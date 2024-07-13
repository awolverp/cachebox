use crate::basic::HashablePyObject;
use crate::{create_pyerr, make_eq_func, make_hasher_func, pickle_get_first_objects};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;
use std::collections::VecDeque;
use std::time;

pub struct TTLValue(pub PyObject, pub time::SystemTime);

impl TTLValue {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    #[inline]
    #[must_use]
    pub fn new(val: PyObject, ttl: f32) -> Self {
        TTLValue(
            val,
            time::SystemTime::now() + time::Duration::from_secs_f32(ttl),
        )
    }

    #[inline]
    pub fn expired(&self) -> bool {
        time::SystemTime::now() > self.1
    }

    #[inline]
    pub fn remaining(&self) -> f32 {
        self.1
            .duration_since(time::SystemTime::now())
            .map(|x| x.as_secs_f32())
            .unwrap_or(0.0)
    }

    #[inline]
    pub fn timestamp(&self) -> f64 {
        self.1
            .duration_since(time::UNIX_EPOCH)
            .map(|x| x.as_secs_f64())
            .unwrap_or(0.0)
    }

    #[must_use]
    #[inline]
    pub fn from_timestamp(val: PyObject, timestamp: f64) -> Self {
        TTLValue(
            val,
            time::UNIX_EPOCH + time::Duration::from_secs_f64(timestamp),
        )
    }
}

pub struct RawTTLCache {
    table: RawTable<(HashablePyObject, TTLValue)>,
    order: VecDeque<HashablePyObject>,
    pub ttl: f32,
    pub maxsize: NonZeroUsize,
}

impl RawTTLCache {
    /// 1. maxsize
    /// 2. table
    /// 3. capacity
    /// 5. order
    /// 4. ttl
    pub const PICKLE_TUPLE_SIZE: isize = 5;

    #[inline]
    pub fn new(maxsize: usize, ttl: f32, capacity: usize) -> PyResult<Self> {
        if ttl <= 0.0 {
            return Err(create_pyerr!(
                pyo3::exceptions::PyValueError,
                "ttl value cannot be negative or zero"
            ));
        }

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

        Ok(Self {
            table,
            maxsize,
            ttl,
            order: VecDeque::new(),
        })
    }

    #[inline]
    pub fn popitem(&mut self) -> PyResult<(HashablePyObject, TTLValue)> {
        match self.order.pop_front() {
            Some(x) => {
                #[cfg(debug_assertions)]
                let val = self.table.remove_entry(x.hash, make_eq_func!(x)).unwrap();

                #[cfg(not(debug_assertions))]
                let val = unsafe {
                    self.table
                        .remove_entry(x.hash, make_eq_func!(x))
                        .unwrap_unchecked()
                };

                Ok(val)
            }
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError)),
        }
    }

    #[inline]
    pub fn expire(&mut self) {
        while let Some(key) = self.order.front() {
            match self.table.find(key.hash, make_eq_func!(key)) {
                Some(x) => {
                    let (_, v) = unsafe { x.as_ref() };
                    if !v.expired() {
                        break;
                    }
                }
                None => {
                    #[cfg(debug_assertions)]
                    unreachable!("key not found in order vecdeque: {:?}", key);

                    #[cfg(not(debug_assertions))]
                    unsafe {
                        core::hint::unreachable_unchecked()
                    }
                }
            }

            self.table.remove_entry(key.hash, make_eq_func!(key));
            self.order.pop_front();
        }
    }

    #[inline]
    pub unsafe fn insert_unchecked(&mut self, key: HashablePyObject, value: TTLValue) -> bool {
        match self
            .table
            .find_or_find_insert_slot(key.hash, make_eq_func!(key), make_hasher_func!())
        {
            Ok(bucket) => {
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().1 }, value);
                false
            }
            Err(slot) => unsafe {
                self.table
                    .insert_in_slot(key.hash, slot, (key.clone(), value));
                true
            },
        }
    }

    #[inline]
    pub fn insert(&mut self, key: HashablePyObject, value: PyObject) -> PyResult<()> {
        if self.table.len() >= self.maxsize.get()
            && self.table.find(key.hash, make_eq_func!(key)).is_none()
        {
            #[cfg(debug_assertions)]
            self.popitem().unwrap();

            #[cfg(not(debug_assertions))]
            unsafe {
                self.popitem().unwrap_unchecked()
            };
        }

        unsafe {
            if self.insert_unchecked(key.clone(), TTLValue::new(value, self.ttl)) {
                self.order.push_back(key);
            }
        }
        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &HashablePyObject) -> Option<&TTLValue> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                let (_, val) = unsafe { bucket.as_ref() };

                if val.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(HashablePyObject, TTLValue)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                let (key, _) = unsafe { bucket.as_ref() };

                #[cfg(debug_assertions)]
                let index = self.order.iter().position(|x| x.eq(key)).unwrap();

                #[cfg(not(debug_assertions))]
                let index = unsafe { self.order.iter().position(|x| x.eq(key)).unwrap_unchecked() };

                self.order.remove(index);

                let (val, _) = unsafe { self.table.remove(bucket) };
                if val.1.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn order_ref(&self) -> &VecDeque<HashablePyObject> {
        &self.order
    }

    #[inline]
    pub fn order_mut(&mut self) -> &mut VecDeque<HashablePyObject> {
        &mut self.order
    }

    #[inline]
    pub fn contains_key(&self, key: &HashablePyObject) -> bool {
        if self.table.is_empty() {
            return false;
        }

        self.table
            .find(key.hash, make_eq_func!(key))
            .filter(|bucket| {
                let (_, v) = unsafe { bucket.as_ref() };
                !v.expired()
            })
            .is_some()
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
    pub fn first(&self) -> Option<&HashablePyObject> {
        self.order.front()
    }

    #[inline]
    pub fn last(&self) -> Option<&HashablePyObject> {
        self.order.back()
    }
}

impl AsRef<RawTable<(HashablePyObject, TTLValue)>> for RawTTLCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(HashablePyObject, TTLValue)> {
        &self.table
    }
}

impl AsMut<RawTable<(HashablePyObject, TTLValue)>> for RawTTLCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(HashablePyObject, TTLValue)> {
        &mut self.table
    }
}

impl crate::basic::PickleMethods for RawTTLCache {
    unsafe fn dumps(&self) -> *mut pyo3::ffi::PyObject {
        let dict = pyo3::ffi::PyDict_New();

        for pair in self.table.iter() {
            let (key, val) = pair.as_ref();

            let val_tuple = pyo3::ffi::PyTuple_New(2);
            let timestamp = pyo3::ffi::PyFloat_FromDouble(val.timestamp());

            pyo3::ffi::PyTuple_SetItem(val_tuple, 0, val.0.as_ptr());
            pyo3::ffi::PyTuple_SetItem(val_tuple, 1, timestamp);

            pyo3::ffi::PyDict_SetItem(dict, key.object.as_ptr(), val_tuple);
            pyo3::ffi::Py_XDECREF(val_tuple);
        }

        let order = pyo3::ffi::PyTuple_New(self.order.len() as isize);
        for (index, key) in self.order.iter().enumerate() {
            pyo3::ffi::PyTuple_SetItem(order, index as isize, key.object.as_ptr());
        }

        let maxsize = pyo3::ffi::PyLong_FromSize_t(self.maxsize.get());
        let capacity = pyo3::ffi::PyLong_FromSize_t(self.table.capacity());
        let ttl = pyo3::ffi::PyFloat_FromDouble(self.ttl as f64);

        let tuple = pyo3::ffi::PyTuple_New(Self::PICKLE_TUPLE_SIZE);
        pyo3::ffi::PyTuple_SetItem(tuple, 0, maxsize);
        pyo3::ffi::PyTuple_SetItem(tuple, 1, dict);
        pyo3::ffi::PyTuple_SetItem(tuple, 2, capacity);
        pyo3::ffi::PyTuple_SetItem(tuple, 3, order);
        pyo3::ffi::PyTuple_SetItem(tuple, 4, ttl);

        tuple
    }

    unsafe fn loads(
        &mut self,
        state: *mut pyo3::ffi::PyObject,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<()> {
        let (maxsize, iterable, capacity) = pickle_get_first_objects!(py, state);

        let order = {
            let obj = pyo3::ffi::PyTuple_GetItem(state, 3);

            if pyo3::ffi::PyTuple_CheckExact(obj) != 1 {
                return Err(create_pyerr!(
                    pyo3::exceptions::PyTypeError,
                    "the order object is not an tuple"
                ));
            }

            obj
        };

        let ttl = {
            let obj = pyo3::ffi::PyTuple_GetItem(state, 4);
            pyo3::ffi::PyFloat_AsDouble(obj) as f32
        };

        if let Some(e) = pyo3::PyErr::take(py) {
            return Err(e);
        }

        let mut new = Self::new(maxsize, ttl, capacity)?;

        #[cfg(debug_assertions)]
        let dict: &Bound<pyo3::types::PyDict> = iterable.downcast_bound(py)?;
        #[cfg(not(debug_assertions))]
        let dict: &Bound<pyo3::types::PyDict> = iterable.downcast_bound(py).unwrap_unchecked();

        let tuple_length = pyo3::ffi::PyTuple_Size(order);

        if tuple_length as usize != dict.len() {
            return Err(create_pyerr!(
                pyo3::exceptions::PyValueError,
                "tuple size isn't equal to dict size"
            ));
        }

        for (key, value) in dict.iter() {
            let value_as_tuple = value.as_ptr();

            if pyo3::ffi::PyTuple_CheckExact(value_as_tuple) != 1 {
                return Err(create_pyerr!(
                    pyo3::exceptions::PyTypeError,
                    "a value in dictionary is not tuple"
                ));
            }

            if pyo3::ffi::PyTuple_Size(value_as_tuple) != 2 {
                return Err(create_pyerr!(
                    pyo3::exceptions::PyTypeError,
                    "a value in dictionary that's tuple, but its size isn't equal 2"
                ));
            }

            let value_0 = pyo3::ffi::PyTuple_GetItem(value_as_tuple, 0);
            let value_1 = pyo3::ffi::PyTuple_GetItem(value_as_tuple, 1);
            let timestamp = pyo3::ffi::PyFloat_AsDouble(value_1);

            // SAFETY: key is hashable, so don't worry
            let hashable = HashablePyObject::try_from_bound(key).unwrap_unchecked();

            new.insert_unchecked(
                hashable,
                TTLValue::from_timestamp(PyObject::from_borrowed_ptr(py, value_0), timestamp),
            );
        }

        if new.order.try_reserve(tuple_length as usize).is_err() {
            return Err(create_pyerr!(pyo3::exceptions::PyMemoryError));
        }

        for k in 0..tuple_length {
            let key = pyo3::ffi::PyTuple_GetItem(order, k);
            let hashable =
                HashablePyObject::try_from_pyobject(PyObject::from_borrowed_ptr(py, key), py)?;
            new.order.push_back(hashable);
        }

        new.expire();

        *self = new;

        Ok(())
    }
}
