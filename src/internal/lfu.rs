//! The LFU Policy

use std::ptr::NonNull;

use crate::hashedkey::HashedKey;
use crate::sorted_heap::{Entry, Iter, SortedHeap};
use hashbrown::raw::RawTable;

macro_rules! compare_fn {
    () => {
        |a, b| a.2.cmp(&b.2)
    };
}

pub struct LFUPolicy {
    pub table: RawTable<NonNull<Entry<(HashedKey, pyo3::PyObject, usize)>>>,
    pub heap: SortedHeap<(HashedKey, pyo3::PyObject, usize)>,
    pub maxsize: core::num::NonZeroUsize,
}

impl LFUPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            heap: SortedHeap::new(),
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
            |node| (*node.as_ptr()).as_ref().0 == key,
            |node| (*node.as_ptr()).as_ref().0.hash,
        ) {
            Ok(bucket) => {
                let node = bucket.as_mut();

                (node.as_mut()).as_mut().2 += 1;
                let oldval = core::mem::replace(&mut (node.as_mut()).as_mut().1, value);

                Some(oldval)
            }
            Err(slot) => {
                // copy key hash
                let hash = key.hash;

                let node = self.heap.push((key, value, default_frequency));
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
        self.heap.sort(compare_fn!());
        let first = self.heap.0.first()?;

        unsafe {
            self.table
                .remove_entry((*first.as_ptr()).as_ref().0.hash, |node| {
                    core::ptr::eq(node.as_ptr(), first.as_ptr())
                })
                .expect("popitem key not found.");
        }

        Some(self.heap.pop_front(compare_fn!()).unwrap())
    }

    #[inline]
    pub fn get(&mut self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).as_ref().0 == *key)
        } {
            Some(bucket) => {
                let node = unsafe { bucket.as_mut() };

                unsafe {
                    (node.as_mut()).as_mut().2 += 1;
                }

                self.heap.1 = false;

                Some(unsafe { &(*node.as_ptr()).as_ref().1 })
            }
            None => None,
        }
    }

    #[inline]
    pub fn peek(&self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).as_ref().0 == *key)
        } {
            Some(bucket) => {
                let node = unsafe { bucket.as_ref() };

                Some(unsafe { &(*node.as_ptr()).as_ref().1 })
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<(HashedKey, pyo3::PyObject, usize)> {
        match unsafe {
            self.table
                .remove_entry(key.hash, |node| (*node.as_ptr()).as_ref().0 == *key)
        } {
            Some(node) => Some(self.heap.remove(node, compare_fn!())),
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).as_ref().0 == *key)
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
            .shrink_to(0, |node| unsafe { (*node.as_ptr()).as_ref().0.hash });
        self.heap.0.shrink_to_fit();
    }

    pub fn iter(&mut self) -> Iter<(HashedKey, pyo3::PyObject, usize)> {
        self.heap.sort(compare_fn!());
        self.heap.iter()
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub unsafe fn to_pickle(
        &mut self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        self.heap.sort(compare_fn!());

        let list = pyo3::ffi::PyList_New(0);
        if list.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for ptr in self.heap.iter() {
            let node = &(*ptr.as_ptr());

            let frequency = pyo3::ffi::PyLong_FromSize_t(node.as_ref().2);
            if frequency.is_null() {
                pyo3::ffi::Py_DECREF(list);
                return Err(pyo3::PyErr::fetch(py));
            }

            let tp = tuple!(
                py,
                3,
                0 => node.as_ref().0.key.clone_ref(py).as_ptr(),
                1 => node.as_ref().1.clone_ref(py).as_ptr(),
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

            // SAFETY: we don't need to check maxsize, we sure `len(iterable) <= maxsize`
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

                let node2 = other.table.get((*node1.as_ptr()).as_ref().0.hash, |x| {
                    (*x.as_ptr()).as_ref().0 == (*node1.as_ptr()).as_ref().0
                });
                if node2.is_none() {
                    return false;
                }

                let node2 = node2.unwrap_unchecked();

                if (*node1.as_ptr()).as_ref().0.hash != (*node2.as_ptr()).as_ref().0.hash
                    || !pyobject_eq!(
                        (*node1.as_ptr()).as_ref().0.key,
                        (*node2.as_ptr()).as_ref().0.key
                    )
                    || !pyobject_eq!((*node1.as_ptr()).as_ref().1, (*node2.as_ptr()).as_ref().1)
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
