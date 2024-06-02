use crate::basic::HashablePyObject;
use crate::create_pyerr;
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;

pub struct RawLFUCache {
    table: RawTable<(HashablePyObject, PyObject, usize)>,
    pub maxsize: NonZeroUsize,
}

impl RawLFUCache {
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
    pub fn popitem(&mut self) -> PyResult<(HashablePyObject, PyObject)> {
        if self.table.is_empty() {
            return Err(create_pyerr!(pyo3::exceptions::PyKeyError));
        }

        let mut vector: Vec<_> = unsafe {
            self.table
                .iter()
                .map(|bucket| {
                    let (_, _, n) = bucket.as_ref();
                    (*n, bucket)
                })
                .collect()
        };
        vector.sort_unstable_by(|(n, _), (m, _)| m.cmp(n));

        #[cfg(debug_assertions)]
        let (_, least_frequently_used_bucket) = vector.pop().unwrap();

        #[cfg(not(debug_assertions))]
        let (_, least_frequently_used_bucket) = unsafe { vector.pop().unwrap_unchecked() };

        let (val, _) = unsafe { self.table.remove(least_frequently_used_bucket) };
        Ok((val.0, val.1))
    }

    #[inline]
    pub unsafe fn insert_unchecked(&mut self, key: HashablePyObject, value: PyObject) {
        match self.table.find_or_find_insert_slot(
            key.hash,
            |(x, _, _)| x.eq(&key),
            |(x, _, _)| x.hash,
        ) {
            Ok(bucket) => {
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().1 }, value);
                bucket.as_mut().2 += 1;
            }
            Err(slot) => unsafe {
                self.table
                    .insert_in_slot(key.hash, slot, (key, value, 0usize));
            },
        }
    }

    #[inline]
    pub fn insert(&mut self, key: HashablePyObject, value: PyObject) -> PyResult<()> {
        if self.table.len() >= self.maxsize.get()
            && self.table.find(key.hash, |(x, _, _)| x.eq(&key)).is_none()
        {
            #[cfg(debug_assertions)]
            self.popitem().unwrap();

            #[cfg(not(debug_assertions))]
            unsafe {
                self.popitem().unwrap_unchecked()
            };
        }

        unsafe {
            self.insert_unchecked(key, value);
        }

        Ok(())
    }

    #[inline]
    pub fn get(&mut self, key: &HashablePyObject) -> Option<&PyObject> {
        if self.table.is_empty() {
            return None;
        }

        self.table
            .find(key.hash, |(x, _, _)| x.eq(key))
            .map(|bucket| {
                let (_, val, n) = unsafe { bucket.as_mut() };
                *n += 1;
                val as &PyObject
            })
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(HashablePyObject, PyObject)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, |(x, _, _)| x.eq(key)) {
            Some(bucket) => {
                let (val, _) = unsafe { self.table.remove(bucket) };
                Some((val.0, val.1))
            }
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashablePyObject) -> bool {
        if self.table.is_empty() {
            return false;
        }

        self.table.find(key.hash, |(x, _, _)| x.eq(key)).is_some()
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

    #[inline]
    pub fn least_frequently_used(&self) -> Option<&HashablePyObject> {
        if self.table.is_empty() {
            return None;
        }

        let mut vector: Vec<_> = unsafe {
            self.table
                .iter()
                .map(|bucket| {
                    let (_, _, n) = bucket.as_ref();
                    (*n, bucket)
                })
                .collect()
        };
        vector.sort_unstable_by(|(n, _), (m, _)| m.cmp(n));

        #[cfg(debug_assertions)]
        let (_, least_frequently_used_bucket) = vector.pop().unwrap();

        #[cfg(not(debug_assertions))]
        let (_, least_frequently_used_bucket) = unsafe { vector.pop().unwrap_unchecked() };

        let (h, _, _) = unsafe { least_frequently_used_bucket.as_ref() };
        Some(h)
    }
}

impl AsRef<RawTable<(HashablePyObject, PyObject, usize)>> for RawLFUCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(HashablePyObject, PyObject, usize)> {
        &self.table
    }
}

impl AsMut<RawTable<(HashablePyObject, PyObject, usize)>> for RawLFUCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(HashablePyObject, PyObject, usize)> {
        &mut self.table
    }
}
