//! The LRU Policy

use crate::hashedkey::HashedKey;
use crate::linked_list;
use hashbrown::raw::RawTable;

pub struct LRUPolicy {
    pub table: RawTable<std::ptr::NonNull<linked_list::Node>>,
    pub list: linked_list::LinkedList,
    pub maxsize: core::num::NonZeroUsize,
}

impl LRUPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            list: linked_list::LinkedList::new(),
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
    ) -> Option<pyo3::PyObject> {
        match self.table.find_or_find_insert_slot(
            key.hash,
            |node| (*node.as_ptr()).element.0 == key,
            |node| (*node.as_ptr()).element.0.hash,
        ) {
            Ok(bucket) => {
                let node = bucket.as_mut();

                let oldval = core::mem::replace(&mut (node.as_mut()).element.1, value);
                self.list.move_back(*node);

                Some(oldval)
            }
            Err(slot) => {
                // copy key hash
                let hash = key.hash;

                let node = self.list.push_back(key, value);
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
            // #[cfg(debug_assertions)]
            self.popitem().unwrap();

            // #[cfg(not(debug_assertions))]
            // unsafe {
            //     self.popitem().unwrap_unchecked();
            // }
        }

        unsafe { self.insert_unchecked(key, value) }
    }

    #[inline]
    pub fn popitem(&mut self) -> Option<(HashedKey, pyo3::PyObject)> {
        let ret = self.list.head?;

        unsafe {
            self.table
                .remove_entry((*ret.as_ptr()).element.0.hash, |node| {
                    core::ptr::eq(node.as_ptr(), ret.as_ptr())
                })
                .expect("popitem key not found.");
        }

        Some(self.list.pop_front().unwrap())
    }

    #[inline]
    pub fn get(&mut self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).element.0 == *key)
        } {
            Some(bucket) => {
                let node = unsafe { bucket.as_mut() };

                unsafe {
                    self.list.move_back(*node);
                }

                Some(unsafe { &(*node.as_ptr()).element.1 })
            }
            None => None,
        }
    }

    #[inline]
    pub fn peek(&self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).element.0 == *key)
        } {
            Some(bucket) => {
                let node = unsafe { bucket.as_ref() };

                Some(unsafe { &(*node.as_ptr()).element.1 })
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<(HashedKey, pyo3::PyObject)> {
        match unsafe {
            self.table
                .remove_entry(key.hash, |node| (*node.as_ptr()).element.0 == *key)
        } {
            Some(node) => Some(unsafe { self.list.remove(node) }),
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        unsafe {
            self.table
                .find(key.hash, |node| (*node.as_ptr()).element.0 == *key)
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
            for pair in iterable.bind(py).try_iter()? {
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
            .shrink_to(0, |node| unsafe { (*node.as_ptr()).element.0.hash })
    }

    #[inline]
    pub unsafe fn to_pickle(
        &self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let list = pyo3::ffi::PyList_New(0);
        if list.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for node in self.list.iter() {
            let (hk, val) = &(*node.as_ptr()).element;

            let tp = tuple!(
                py,
                2,
                0 => hk.key.clone_ref(py).as_ptr(),
                1 => val.clone_ref(py).as_ptr(),
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
        tuple!(check state, size=3)?;
        let (maxsize, iterable, capacity) = extract_pickle_tuple!(py, state);

        let mut new = Self::new(maxsize, capacity)?;
        new.update(py, iterable)?;

        *self = new;
        Ok(())
    }
}

impl PartialEq for LRUPolicy {
    fn eq(&self, other: &Self) -> bool {
        if self.maxsize != other.maxsize {
            return false;
        }

        if self.list.len() != other.list.len() {
            return false;
        }

        for (node1, node2) in self.list.iter().zip(other.list.iter()) {
            let (key1, val1) = unsafe { &(*node1.as_ptr()).element };
            let (key2, val2) = unsafe { &(*node2.as_ptr()).element };

            if key1.hash != key2.hash
                || !pyobject_eq!(key1.key, key2.key)
                || !pyobject_eq!(val1, val2)
            {
                return false;
            }
        }

        true
    }
}

impl Eq for LRUPolicy {}

// because we use it in Mutex
unsafe impl Sync for LRUPolicy {}

// because we use it in Mutex
unsafe impl Send for LRUPolicy {}
