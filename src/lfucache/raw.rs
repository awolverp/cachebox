use crate::basic::HashablePyObject;
use crate::{create_pyerr, pickle_get_first_objects};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;

pub struct RawLFUCache {
    table: RawTable<(HashablePyObject, PyObject, usize)>,
    pub maxsize: NonZeroUsize,
}

impl RawLFUCache {
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
    pub unsafe fn insert_unchecked(&mut self, key: HashablePyObject, value: PyObject, n: usize) {
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
                self.table.insert_in_slot(key.hash, slot, (key, value, n));
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
            self.insert_unchecked(key, value, 0);
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
    pub fn peek(&self, key: &HashablePyObject) -> Option<&PyObject> {
        if self.table.is_empty() {
            return None;
        }

        self.table
            .find(key.hash, |(x, _, _)| x.eq(key))
            .map(|bucket| {
                let (_, val, _) = unsafe { bucket.as_ref() };
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

    #[inline]
    pub fn least_frequently_used(&self, n: usize) -> Option<&HashablePyObject> {
        if self.table.is_empty() || self.table.len() <= n {
            return None;
        }

        let mut vector: Vec<_> = unsafe {
            self.table
                .iter()
                .map(|bucket| {
                    let (_, _, n) = bucket.as_ref();
                    (std::cmp::Reverse(*n), bucket)
                })
                .collect()
        };
        vector.sort_unstable_by(|(n, _), (m, _)| m.cmp(n));

        let (_, least_frequently_used_bucket) = vector.swap_remove(n);

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

impl crate::basic::PickleMethods for RawLFUCache {
    unsafe fn dumps(&self) -> *mut pyo3::ffi::PyObject {
        // {key: (val, count)}
        let dict = pyo3::ffi::PyDict_New();

        for pair in self.table.iter() {
            let (key, val, count) = pair.as_ref();

            let val_tuple = pyo3::ffi::PyTuple_New(2);
            let c = pyo3::ffi::PyLong_FromSize_t(*count);
            pyo3::ffi::PyTuple_SetItem(val_tuple, 0, val.as_ptr());
            pyo3::ffi::PyTuple_SetItem(val_tuple, 1, c);

            pyo3::ffi::PyDict_SetItem(dict, key.object.as_ptr(), val_tuple);
            pyo3::ffi::Py_XDECREF(val_tuple);
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
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<()> {
        let (maxsize, iterable, capacity) = pickle_get_first_objects!(py, state);

        let mut new = Self::new(maxsize, capacity)?;

        #[cfg(debug_assertions)]
        let dict: &Bound<pyo3::types::PyDict> = iterable.downcast_bound(py)?;
        #[cfg(not(debug_assertions))]
        let dict: &Bound<pyo3::types::PyDict> = iterable.downcast_bound(py).unwrap_unchecked();

        for (key, value) in dict.iter() {
            // SAFETY: key is hashable, so don't worry
            let hashable = HashablePyObject::try_from_bound(key).unwrap_unchecked();

            let op = value.as_ptr();

            if pyo3::ffi::PyTuple_CheckExact(op) != 1 || pyo3::ffi::PyTuple_Size(op) != 2 {
                return Err(create_pyerr!(
                    pyo3::exceptions::PyTypeError,
                    "expected tuple, found another type #op"
                ));
            }

            let val = pyo3::ffi::PyTuple_GetItem(op, 0);

            let n = {
                let obj = pyo3::ffi::PyTuple_GetItem(op, 1);
                pyo3::ffi::PyLong_AsSize_t(obj)
            };

            new.insert_unchecked(hashable, PyObject::from_borrowed_ptr(py, val), n);
        }

        *self = new;

        Ok(())
    }
}
