//! The LFU Policy

use std::ptr::NonNull;

use crate::hashedkey::HashedKey;
use hashbrown::raw::RawTable;

pub struct LFUPolicy {
    pub table: RawTable<NonNull<LFUNode>>,
    pub heap: LFUHeap,
    pub maxsize: core::num::NonZeroUsize,
}

pub struct LFUHeap(Vec<NonNull<LFUNode>>, bool);

pub struct LFUNode {
    pub key: HashedKey,
    pub value: pyo3::PyObject,
    pub frequency: usize,
}

pub struct LFUPtrIter {
    slice: *const NonNull<LFUNode>,
    index: usize,
    len: usize,
}

impl LFUPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            heap: LFUHeap::new(),
            maxsize,
        })
    }

    /// # Safety
    ///
    /// This method is unsafe because does not checks the maxsize and this
    /// may occurred errors and bad situations in future if you don't care about
    /// maxsize.
    #[inline]
    unsafe fn insert_unchecked(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
        default_frequency: usize,
    ) -> Option<pyo3::PyObject> {
        match self.table.find_or_find_insert_slot(
            key.hash,
            |node| (*node.as_ptr()).key == key,
            |node| (*node.as_ptr()).key.hash,
        ) {
            Ok(bucket) => {
                let node = bucket.as_mut();

                (node.as_mut()).frequency += 1;
                let oldval = core::mem::replace(&mut (node.as_mut()).value, value);

                Some(oldval)
            }
            Err(slot) => {
                // copy key hash
                let hash = key.hash;

                let node = self.heap.push(key, value, default_frequency);
                unsafe {
                    self.table.insert_in_slot(hash, slot, node);
                }

                None
            }
        }
    }

    #[inline]
    pub fn insert(&mut self, key: HashedKey, value: pyo3::PyObject) -> Option<pyo3::PyObject> {
        if self.table.len() >= self.maxsize.get() && !self.contains_key(&key) {
            self.popitem().unwrap();
        }

        unsafe { self.insert_unchecked(key, value, 1) }
    }

    #[inline]
    pub fn popitem(&mut self) -> Option<(HashedKey, pyo3::PyObject, usize)> {
        self.heap.sort();
        let first = self.heap.0.first()?;

        unsafe {
            self.table
                .remove_entry((*first.as_ptr()).key.hash, |node| {
                    core::ptr::eq(node.as_ptr(), first.as_ptr())
                })
                .expect("popitem key not found.");
        }

        Some(self.heap.pop_front().unwrap())
    }

    #[inline]
    pub fn get(&mut self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).key == *key)
        } {
            Some(bucket) => {
                let node = unsafe { bucket.as_mut() };

                unsafe {
                    (node.as_mut()).frequency += 1;
                }

                self.heap.1 = false;

                Some(unsafe { &(*node.as_ptr()).value })
            }
            None => None,
        }
    }

    #[inline]
    pub fn peek(&self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).key == *key)
        } {
            Some(bucket) => {
                let node = unsafe { bucket.as_ref() };

                Some(unsafe { &(*node.as_ptr()).value })
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<(HashedKey, pyo3::PyObject, usize)> {
        match unsafe {
            self.table
                .remove_entry(key.hash, |node| (*node.as_ptr()).key == *key)
        } {
            Some(node) => Some(self.heap.remove(node)),
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).key == *key)
                .is_some()
        }
    }

    #[inline]
    pub fn update(&mut self, py: pyo3::Python<'_>, iterable: pyo3::PyObject) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe {
                iterable
                    .downcast_bound::<pyo3::types::PyDict>(py)
                    .unwrap_unchecked()
            };

            for (key, value) in dict.iter() {
                let hk = unsafe { HashedKey::from_pyobject(py, key.unbind()).unwrap_unchecked() };
                self.insert(hk, value.unbind());
            }

            Ok(())
        } else {
            for pair in iterable.bind(py).iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = HashedKey::from_pyobject(py, key)?;
                self.insert(hk, value);
            }

            Ok(())
        }
    }

    #[inline(always)]
    pub fn shrink_to_fit(&mut self) {
        self.table
            .shrink_to(0, |node| unsafe { (*node.as_ptr()).key.hash })
    }

    pub fn iter(&mut self) -> LFUPtrIter {
        self.heap.sort();

        LFUPtrIter {
            slice: self.heap.0.as_ptr(),
            index: 0,
            len: self.heap.len(),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub unsafe fn to_pickle(
        &mut self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        self.heap.sort();

        let list = pyo3::ffi::PyList_New(0);
        if list.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for ptr in self.heap.0.iter() {
            let node = &(*ptr.as_ptr());

            let frequency = pyo3::ffi::PyLong_FromSize_t(node.frequency);
            if frequency.is_null() {
                pyo3::ffi::Py_DECREF(list);
                return Err(pyo3::PyErr::fetch(py));
            }

            let tp = tuple!(
                py,
                3,
                0 => node.key.key.clone_ref(py).as_ptr(),
                1 => node.value.clone_ref(py).as_ptr(),
                2 => frequency,
            );

            if let Err(x) = tp {
                pyo3::ffi::Py_DECREF(list);
                return Err(x);
            }

            if pyo3::ffi::PyList_Append(list, tp.unwrap_unchecked()) == -1 {
                pyo3::ffi::Py_DECREF(list);
                return Err(pyo3::PyErr::fetch(py));
            }
        }

        let maxsize = pyo3::ffi::PyLong_FromSize_t(self.maxsize.get());
        let capacity = pyo3::ffi::PyLong_FromSize_t(self.table.capacity());

        tuple!(
            py,
            3,
            0 => maxsize,
            1 => list,
            2 => capacity,
        )
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub unsafe fn from_pickle(
        &mut self,
        py: pyo3::Python<'_>,
        state: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::PyAnyMethods;

        tuple!(check state, size=3)?;
        let (maxsize, iterable, capacity) = extract_pickle_tuple!(py, state);

        // SAFETY: we check `iterable` type in `extract_pickle_tuple` macro
        if maxsize < (pyo3::ffi::PyObject_Size(iterable.as_ptr()) as usize) {
            return Err(err!(
                pyo3::exceptions::PyValueError,
                "iterable object size is greater than maxsize"
            ));
        }

        let mut new = Self::new(maxsize, capacity)?;

        for pair in iterable.bind(py).iter()? {
            let (key, value, fr) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject, usize)>()?;

            let hk = HashedKey::from_pyobject(py, key)?;
            new.insert_unchecked(hk, value, fr);
        }

        *self = new;
        Ok(())
    }
}

impl PartialEq for LFUPolicy {
    fn eq(&self, other: &Self) -> bool {
        if self.maxsize != other.maxsize {
            return false;
        }

        if self.heap.len() != other.heap.len() {
            return false;
        }

        unsafe {
            for bucket in self.table.iter() {
                let node1 = bucket.as_ref();

                let node2 = other.table.get((*node1.as_ptr()).key.hash, |x| {
                    (*x.as_ptr()).key == (*node1.as_ptr()).key
                });
                if node2.is_none() {
                    return false;
                }

                let node2 = node2.unwrap_unchecked();

                if (*node1.as_ptr()).key.hash != (*node2.as_ptr()).key.hash
                    || !pyobject_eq!((*node1.as_ptr()).key.key, (*node2.as_ptr()).key.key)
                    || !pyobject_eq!((*node1.as_ptr()).value, (*node2.as_ptr()).value)
                {
                    return false;
                }
            }
        }

        true
    }
}

impl Eq for LFUPolicy {}

// because we use it in Mutex
unsafe impl Sync for LFUPolicy {}

// because we use it in Mutex
unsafe impl Send for LFUPolicy {}

impl LFUHeap {
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new(), true)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
        frequency: usize,
    ) -> NonNull<LFUNode> {
        unsafe {
            let node = NonNull::new_unchecked(Box::into_raw(Box::new(LFUNode {
                key,
                value,
                frequency,
            })));

            self.0.push(node);
            self.1 = false;

            node
        }
    }

    #[inline]
    pub fn sort(&mut self) {
        if !self.1 {
            unsafe {
                if self.0.len() > 1 {
                    self.0
                        .sort_by(|a, b| (*a.as_ptr()).frequency.cmp(&(*b.as_ptr()).frequency));
                }
            }

            self.1 = true;
        }
    }

    pub fn pop_front(&mut self) -> Option<(HashedKey, pyo3::PyObject, usize)> {
        if self.0.is_empty() {
            return None;
        }

        self.sort();

        let node = self.0.remove(0);
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        Some((boxed_node.key, boxed_node.value, boxed_node.frequency))
    }

    pub fn pop_back(&mut self) -> Option<(HashedKey, pyo3::PyObject, usize)> {
        self.sort();

        let node = self.0.pop()?;
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        Some((boxed_node.key, boxed_node.value, boxed_node.frequency))
    }

    pub fn get(&self, index: usize) -> Option<&NonNull<LFUNode>> {
        self.0.get(index)
    }

    pub fn remove(&mut self, node: NonNull<LFUNode>) -> (HashedKey, pyo3::PyObject, usize) {
        debug_assert!(!self.0.is_empty());

        if self.0.len() == 1 {
            return self.pop_back().unwrap();
        }

        self.sort();

        let index = unsafe {
            let greater = (*self.0[self.0.len() - 1].as_ptr()).frequency;

            if (greater / 2) >= (*node.as_ptr()).frequency {
                self.0.iter().position(|x| node == *x).unwrap()
            } else {
                self.0.iter().rposition(|x| node == *x).unwrap()
            }
        };

        let node = self.0.remove(index);
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        (boxed_node.key, boxed_node.value, boxed_node.frequency)
    }

    pub fn clear(&mut self) {
        while self.pop_back().is_some() {}
    }
}

impl Drop for LFUHeap {
    fn drop(&mut self) {
        struct DropGuard<'a>(&'a mut LFUHeap);

        impl<'a> Drop for DropGuard<'a> {
            fn drop(&mut self) {
                // Continue the same loop we do below. This only runs when a destructor has
                // panicked. If another one panics this will abort.
                while self.0.pop_back().is_some() {}
            }
        }

        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard(self);
        while guard.0.pop_back().is_some() {}
        core::mem::forget(guard);
    }
}

impl Iterator for LFUPtrIter {
    type Item = NonNull<LFUNode>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            None
        } else {
            let value = unsafe { self.slice.add(self.index) };
            self.index += 1;
            Some(unsafe { *value })
        }
    }
}

// because we use it in Mutex
unsafe impl Sync for LFUPtrIter {}

// because we use it in Mutex
unsafe impl Send for LFUPtrIter {}
