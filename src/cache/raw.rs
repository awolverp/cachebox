use crate::{basic::HashablePyObject, create_pyerr, make_eq_func, make_hasher_func};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;

pub struct RawCache {
    table: RawTable<(HashablePyObject, PyObject)>,
    pub maxsize: NonZeroUsize,
}

impl RawCache {
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
    pub fn extend_from_dict(&mut self, dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        for (key, value) in dict.iter() {
            let hashable = HashablePyObject::try_from_bound(key)?;
            self.insert(hashable, value.unbind())?;
        }

        Ok(())
    }

    #[inline]
    pub fn extend_from_iter(
        &mut self,
        obj: pyo3::Borrowed<'_, '_, PyAny>,
        py: Python<'_>,
    ) -> PyResult<()> {
        for pair in obj.iter()? {
            let (key, value): (Py<PyAny>, Py<PyAny>) = pair?.extract()?;

            let hashable = HashablePyObject::try_from_pyobject(key, py)?;
            self.insert(hashable, value)?;
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
