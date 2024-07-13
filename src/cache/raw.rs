use crate::basic::{HashablePyObject, PickleMethods};
use crate::{create_pyerr, make_eq_func, make_hasher_func, pickle_get_first_objects};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;

pub struct RawCache {
    table: RawTable<(HashablePyObject, PyObject)>,
    pub maxsize: NonZeroUsize,
}

impl RawCache {
    /// 1. maxsize
    /// 2. table
    /// 3. capacity
    pub const PICKLE_TUPLE_SIZE: isize = 3;

    #[inline]
    pub fn new(maxsize: usize, capacity: usize) -> PyResult<Self> {
        let capacity = {
            if maxsize != 0 {
                core::cmp::min(maxsize, capacity)
            } else {
                capacity
            }
        };

        let maxsize = unsafe {
            NonZeroUsize::new_unchecked(if maxsize == 0 {
                isize::MAX as usize
            } else {
                maxsize
            })
        };

        let table = {
            if capacity > 0 {
                RawTable::try_with_capacity(capacity)
                    .map_err(|_| create_pyerr!(pyo3::exceptions::PyMemoryError))?
            } else {
                RawTable::new()
            }
        };

        Ok(Self { table, maxsize })
    }

    #[inline]
    pub unsafe fn insert_unchecked(&mut self, key: HashablePyObject, value: PyObject) {
        match self
            .table
            .find_or_find_insert_slot(key.hash, make_eq_func!(key), make_hasher_func!())
        {
            Ok(bucket) => {
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().1 }, value);
            }
            Err(slot) => unsafe {
                self.table.insert_in_slot(key.hash, slot, (key, value));
            },
        }
    }

    #[inline]
    pub fn insert(&mut self, key: HashablePyObject, value: PyObject) -> PyResult<()> {
        if self.table.len() >= self.maxsize.get()
            && self.table.find(key.hash, make_eq_func!(key)).is_none()
        {
            return Err(create_pyerr!(
                pyo3::exceptions::PyOverflowError,
                "The cache has reached the maxsize limit"
            ));
        }

        unsafe {
            self.insert_unchecked(key, value);
        }
        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &HashablePyObject) -> Option<&PyObject> {
        if self.table.is_empty() {
            return None;
        }

        self.table.find(key.hash, make_eq_func!(key)).map(|bucket| {
            let (_, val) = unsafe { bucket.as_ref() };
            val
        })
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(HashablePyObject, PyObject)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                let (val, _) = unsafe { self.table.remove(bucket) };
                Some(val)
            }
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashablePyObject) -> bool {
        if self.table.is_empty() {
            return false;
        }

        self.table.find(key.hash, make_eq_func!(key)).is_some()
    }

    #[inline]
    fn extend_from_dict(&mut self, dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        for (key, value) in dict.iter() {
            let hashable = HashablePyObject::try_from_bound(key)?;
            self.insert(hashable, value.unbind())?;
        }

        Ok(())
    }

    #[inline]
    fn extend_from_iter(&mut self, obj: &pyo3::Bound<'_, PyAny>, py: Python<'_>) -> PyResult<()> {
        for pair in obj.iter()? {
            let (key, value): (Py<PyAny>, Py<PyAny>) = pair?.extract()?;

            let hashable = HashablePyObject::try_from_pyobject(key, py)?;
            self.insert(hashable, value)?;
        }

        Ok(())
    }

    pub fn update(&mut self, py: Python<'_>, iterable: PyObject) -> PyResult<()> {
        if unsafe { pyo3::ffi::PyDict_Check(iterable.as_ptr()) == 1 } {
            let dict = iterable.downcast_bound::<pyo3::types::PyDict>(py)?;
            self.extend_from_dict(dict)?;
        } else {
            self.extend_from_iter(iterable.bind(py), py)?;
        }

        Ok(())
    }
}

impl AsRef<RawTable<(HashablePyObject, PyObject)>> for RawCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(HashablePyObject, PyObject)> {
        &self.table
    }
}

impl AsMut<RawTable<(HashablePyObject, PyObject)>> for RawCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(HashablePyObject, PyObject)> {
        &mut self.table
    }
}

impl PickleMethods for RawCache {
    unsafe fn dumps(&self) -> *mut pyo3::ffi::PyObject {
        let dict = pyo3::ffi::PyDict_New();

        for pair in self.table.iter() {
            let (key, val) = pair.as_ref();
            // SAFETY: we don't need to check error because we sure about key that is hashable
            pyo3::ffi::PyDict_SetItem(dict, key.object.as_ptr(), val.as_ptr());
        }

        let maxsize = pyo3::ffi::PyLong_FromSize_t(self.maxsize.get());
        let capacity = pyo3::ffi::PyLong_FromSize_t(self.table.capacity());

        let tuple = pyo3::ffi::PyTuple_New(Self::PICKLE_TUPLE_SIZE);
        pyo3::ffi::PyTuple_SetItem(tuple, 0, maxsize);
        pyo3::ffi::PyTuple_SetItem(tuple, 1, dict);
        pyo3::ffi::PyTuple_SetItem(tuple, 2, capacity);

        tuple
    }

    unsafe fn loads(
        &mut self,
        state: *mut pyo3::ffi::PyObject,
        py: Python<'_>,
    ) -> pyo3::PyResult<()> {
        let (maxsize, iterable, capacity) = pickle_get_first_objects!(py, state);

        let mut new = Self::new(maxsize, capacity)?;

        #[cfg(debug_assertions)]
        new.extend_from_dict(iterable.downcast_bound(py)?)?;
        #[cfg(not(debug_assertions))]
        new.extend_from_dict(iterable.downcast_bound(py).unwrap_unchecked())?;

        *self = new;

        Ok(())
    }
}
