//! The VTTL Policy

use crate::hashedkey::HashedKey;
use crate::sorted_heap;
use hashbrown::raw::RawTable;
use std::ptr::NonNull;
use std::time;

pub struct VTTLElement {
    pub key: HashedKey,
    pub value: pyo3::PyObject,
    pub expire_at: Option<time::SystemTime>,
}

impl VTTLElement {
    #[inline]
    pub fn new(key: HashedKey, value: pyo3::PyObject, ttl: Option<f64>) -> Self {
        Self {
            key,
            value,
            expire_at: ttl
                .map(|secs| time::SystemTime::now() + time::Duration::from_secs_f64(secs)),
        }
    }

    #[inline]
    pub fn reset(&mut self, value: pyo3::PyObject, ttl: Option<f64>) -> pyo3::PyObject {
        self.expire_at =
            ttl.map(|secs| time::SystemTime::now() + time::Duration::from_secs_f64(secs));
        core::mem::replace(&mut self.value, value)
    }

    #[inline]
    pub fn expired(&self) -> bool {
        self.expire_at
            .filter(|x| std::time::SystemTime::now() >= *x)
            .is_some()
    }

    #[inline]
    pub fn or_none(self) -> Option<Self> {
        if self.expired() {
            None
        } else {
            Some(self)
        }
    }

    #[inline]
    pub fn or_none_ref(&self) -> Option<&Self> {
        if self.expired() {
            None
        } else {
            Some(self)
        }
    }
}

pub struct VTTLPolicy {
    pub table: RawTable<NonNull<sorted_heap::Entry<VTTLElement>>>,
    pub heap: sorted_heap::SortedHeap<VTTLElement>,
    pub maxsize: core::num::NonZeroUsize,
}

macro_rules! compare_fn {
    () => {
        |a, b| {
            if a.expire_at.is_none() && b.expire_at.is_none() {
                return std::cmp::Ordering::Equal;
            }
            if b.expire_at.is_none() {
                return std::cmp::Ordering::Less;
            }
            if b.expire_at.is_none() {
                return std::cmp::Ordering::Greater;
            }
            a.expire_at.cmp(&b.expire_at)
        }
    };
}

impl VTTLPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            heap: sorted_heap::SortedHeap::new(),
            maxsize,
        })
    }

    #[inline(always)]
    pub fn expire(&mut self) {
        self.heap.sort(compare_fn!());

        while let Some(x) = self.heap.0.first() {
            unsafe {
                if !(*x.as_ptr()).as_ref().expired() {
                    break;
                }

                self.table
                    .remove_entry((*x.as_ptr()).as_ref().key.hash, |node| node == x)
                    .unwrap();

                self.heap.pop_front(compare_fn!());
            }
        }
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
        ttl: Option<f64>,
    ) -> Option<pyo3::PyObject> {
        match self.table.find_or_find_insert_slot(
            key.hash,
            |node| (*node.as_ptr()).as_ref().key == key,
            |node| (*node.as_ptr()).as_ref().key.hash,
        ) {
            Ok(bucket) => {
                let node = bucket.as_mut();

                let oldval = (*node.as_ptr()).as_mut().reset(value, ttl);
                self.heap.1 = false;

                Some(oldval)
            }
            Err(slot) => {
                // copy key hash
                let hash = key.hash;

                let node = self.heap.push(VTTLElement::new(key, value, ttl));
                unsafe {
                    self.table.insert_in_slot(hash, slot, node);
                }

                self.heap.1 = false;

                None
            }
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
        ttl: Option<f64>,
        expire: bool,
    ) -> Option<pyo3::PyObject> {
        if expire {
            self.expire();
        }

        if self.table.len() >= self.maxsize.get()
            && self
                .table
                .find(key.hash, |node| unsafe {
                    (*node.as_ptr()).as_ref().key == key
                })
                .is_none()
        {
            self.popitem().unwrap();
        }

        unsafe { self.insert_unchecked(key, value, ttl) }
    }

    #[inline]
    pub fn popitem(&mut self) -> Option<VTTLElement> {
        // self.heap.sort(compare_fn!());
        self.expire();

        let first = self.heap.0.first()?;

        unsafe {
            self.table
                .remove_entry((*first.as_ptr()).as_ref().key.hash, |node| {
                    core::ptr::eq(node.as_ptr(), first.as_ptr())
                })
                .expect("popitem key not found.");
        }

        Some(self.heap.pop_front(compare_fn!()).unwrap())
    }

    #[inline]
    pub fn get(&self, key: &HashedKey) -> Option<&VTTLElement> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).as_ref().key == *key)
        } {
            Some(bucket) => unsafe {
                let node = bucket.as_ref();

                let element = (*node.as_ptr()).as_ref();
                element.or_none_ref()
            },
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<VTTLElement> {
        match unsafe {
            self.table
                .remove_entry(key.hash, |node| (*node.as_ptr()).as_ref().key == *key)
        } {
            Some(node) => {
                let element = self.heap.remove(node, compare_fn!());
                element.or_none()
            }
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        unsafe {
            self.table
                .get(key.hash, |node| (*node.as_ptr()).as_ref().key == *key)
                .filter(|node| !(*node.as_ptr()).as_ref().expired())
                .is_some()
        }
    }

    #[inline]
    pub fn update(
        &mut self,
        py: pyo3::Python<'_>,
        iterable: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        self.expire();

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe {
                iterable
                    .downcast_bound::<pyo3::types::PyDict>(py)
                    .unwrap_unchecked()
            };

            for (key, value) in dict.iter() {
                let hk = unsafe { HashedKey::from_pyobject(py, key.unbind()).unwrap_unchecked() };
                self.insert(hk, value.unbind(), ttl, false);
            }

            Ok(())
        } else {
            for pair in iterable.bind(py).try_iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = HashedKey::from_pyobject(py, key)?;
                self.insert(hk, value, ttl, false);
            }

            Ok(())
        }
    }

    #[inline(always)]
    pub fn shrink_to_fit(&mut self) {
        self.table
            .shrink_to(0, |node| unsafe { (*node.as_ptr()).as_ref().key.hash });
        self.heap.0.shrink_to_fit();
    }

    pub fn iter(&mut self) -> sorted_heap::Iter<VTTLElement> {
        self.heap.sort(compare_fn!());
        self.heap.iter()
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub unsafe fn to_pickle(
        &mut self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        self.expire();

        let list = pyo3::ffi::PyList_New(0);
        if list.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for ptr in self.heap.iter() {
            let node = &(*ptr.as_ptr());

            let ttlobject =
                pyo3::ffi::PyLong_FromDouble(node.as_ref().expire_at.map_or(0.0, |x| {
                    x.duration_since(time::UNIX_EPOCH).unwrap().as_secs_f64()
                }));
            if ttlobject.is_null() {
                pyo3::ffi::Py_DECREF(list);
                return Err(pyo3::PyErr::fetch(py));
            }

            let tp = tuple!(
                py,
                3,
                0 => node.as_ref().key.key.clone_ref(py).as_ptr(),
                1 => node.as_ref().value.clone_ref(py).as_ptr(),
                2 => ttlobject,
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

        for pair in iterable.bind(py).try_iter()? {
            let (key, value, timestamp) =
                pair?.extract::<(pyo3::PyObject, pyo3::PyObject, f64)>()?;

            let hk = HashedKey::from_pyobject(py, key)?;

            let ttl = {
                if timestamp == 0.0 {
                    None
                } else {
                    let now = time::SystemTime::now();
                    let as_system_time =
                        time::UNIX_EPOCH + time::Duration::from_secs_f64(timestamp);

                    if now >= as_system_time {
                        // key is expired
                        continue;
                    }

                    Some(as_system_time.duration_since(now).unwrap().as_secs_f64())
                }
            };

            // SAFETY: we don't need to check maxsize, we sure `len(iterable) <= maxsize`
            new.insert_unchecked(hk, value, ttl);
        }

        *self = new;
        Ok(())
    }
}

impl PartialEq for VTTLPolicy {
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

                let node2 = other.table.get((*node1.as_ptr()).as_ref().key.hash, |x| {
                    (*x.as_ptr()).as_ref().key == (*node1.as_ptr()).as_ref().key
                });
                if node2.is_none() {
                    return false;
                }

                let node2 = node2.unwrap_unchecked();

                if (*node1.as_ptr()).as_ref().key.hash != (*node2.as_ptr()).as_ref().key.hash
                    || !pyobject_eq!(
                        (*node1.as_ptr()).as_ref().key.key,
                        (*node2.as_ptr()).as_ref().key.key
                    )
                    || !pyobject_eq!(
                        (*node1.as_ptr()).as_ref().value,
                        (*node2.as_ptr()).as_ref().value
                    )
                {
                    return false;
                }
            }
        }

        true
    }
}

impl Eq for VTTLPolicy {}

// because we use it in Mutex
unsafe impl Sync for VTTLPolicy {}

// because we use it in Mutex
unsafe impl Send for VTTLPolicy {}
