use crate::basic::HashablePyObject;
use crate::{create_pyerr, pickle_get_first_objects};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;
use std::time;

macro_rules! make_eq_func {
    ($key:expr) => {
        |(x, _)| x.key() == $key.key()
    };
}

macro_rules! sort_keys {
    ($order:expr) => {
        $order.sort_unstable_by(|a, b| {
            let ap = a.expiration();
            let bp = b.expiration();

            if ap.is_none() && bp.is_none() {
                return std::cmp::Ordering::Equal;
            }
            if bp.is_none() {
                return std::cmp::Ordering::Greater;
            }
            if ap.is_none() {
                return std::cmp::Ordering::Less;
            }
            bp.cmp(&ap)
        });
    };
}

#[cfg_attr(debug_assertions, derive(Clone, Debug))]
#[cfg_attr(not(debug_assertions), derive(Clone))]
pub enum VTTLKey {
    NoExpire(HashablePyObject),
    Expire(HashablePyObject, time::SystemTime),
}

impl VTTLKey {
    pub const SIZE: usize = core::mem::size_of::<VTTLKey>();

    #[must_use]
    #[inline]
    pub fn new(val: HashablePyObject, ttl: Option<f32>) -> Self {
        match ttl {
            Some(x) => Self::Expire(
                val,
                time::SystemTime::now() + time::Duration::from_secs_f32(x),
            ),
            None => Self::NoExpire(val),
        }
    }

    #[inline]
    pub fn expired(&self) -> bool {
        match *self {
            Self::Expire(_, ref ttl) => time::SystemTime::now() > *ttl,
            Self::NoExpire(_) => false,
        }
    }

    #[inline]
    pub fn key(&self) -> &HashablePyObject {
        match self {
            Self::NoExpire(val) => val,
            Self::Expire(val, _) => val,
        }
    }

    #[inline]
    pub fn into_key(self) -> HashablePyObject {
        match self {
            Self::NoExpire(val) => val,
            Self::Expire(val, _) => val,
        }
    }

    #[inline]
    pub fn expiration(&self) -> Option<&time::SystemTime> {
        match self {
            Self::NoExpire(_) => None,
            Self::Expire(_, instant) => Some(instant),
        }
    }

    #[inline]
    pub fn remaining(&self) -> Option<f32> {
        match self {
            Self::NoExpire(_) => None,
            Self::Expire(_, instant) => Some(
                instant
                    .duration_since(time::SystemTime::now())
                    .map(|x| x.as_secs_f32())
                    .unwrap_or(0.0),
            ),
        }
    }

    #[inline]
    pub fn timestamp(&self) -> Option<f64> {
        match self {
            Self::NoExpire(_) => None,
            Self::Expire(_, instant) => Some(
                instant
                    .duration_since(time::UNIX_EPOCH)
                    .map(|x| x.as_secs_f64())
                    .unwrap_or(0.0),
            ),
        }
    }

    #[must_use]
    #[inline]
    pub fn from_timestamp(val: HashablePyObject, timestamp: Option<f64>) -> Self {
        match timestamp {
            Some(x) => Self::Expire(val, time::UNIX_EPOCH + time::Duration::from_secs_f64(x)),
            None => Self::NoExpire(val),
        }
    }
}

pub struct RawVTTLCache {
    table: RawTable<(VTTLKey, PyObject)>,
    order: Vec<VTTLKey>,
    pub maxsize: NonZeroUsize,
}

impl RawVTTLCache {
    /// 1. maxsize
    /// 2. table
    /// 3. capacity
    /// 4. order
    pub const PICKLE_TUPLE_SIZE: isize = 4;

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

        Ok(Self {
            table,
            maxsize,
            order: Vec::new(),
        })
    }

    #[inline]
    pub fn popitem(&mut self) -> PyResult<(VTTLKey, PyObject)> {
        match self.order.pop() {
            Some(key) => {
                #[cfg(debug_assertions)]
                let val = self
                    .table
                    .remove_entry(key.key().hash, make_eq_func!(key))
                    .unwrap();

                #[cfg(not(debug_assertions))]
                let val = unsafe {
                    self.table
                        .remove_entry(key.key().hash, make_eq_func!(key))
                        .unwrap_unchecked()
                };

                Ok(val)
            }
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError)),
        }
    }

    #[inline]
    pub fn expire(&mut self) {
        while let Some(key) = self.order.last() {
            if !key.expired() {
                break;
            }

            self.table.remove_entry(key.key().hash, make_eq_func!(key));
            self.order.pop();
        }
    }

    pub unsafe fn insert_unchecked(&mut self, key: VTTLKey, value: PyObject) -> bool {
        match self.table.find_or_find_insert_slot(
            key.key().hash,
            |(x, _)| x.key() == key.key(),
            |(x, _)| x.key().hash,
        ) {
            Ok(bucket) => {
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().0 }, key);
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().1 }, value);
                false
            }
            Err(slot) => unsafe {
                self.table
                    .insert_in_slot(key.key().hash, slot, (key, value));
                true
            },
        }
    }

    #[inline]
    pub unsafe fn insert_unsorted(
        &mut self,
        key: HashablePyObject,
        value: PyObject,
        ttl: Option<f32>,
    ) {
        if self.table.len() >= self.maxsize.get()
            && self
                .table
                .find(key.hash, |(x, _)| *x.key() == key)
                .is_none()
        {
            #[cfg(debug_assertions)]
            self.popitem().unwrap();

            #[cfg(not(debug_assertions))]
            unsafe {
                self.popitem().unwrap_unchecked()
            };
        }

        let k = VTTLKey::new(key, ttl);
        if self.insert_unchecked(k.clone(), value) {
            self.order.push(k);
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: HashablePyObject,
        value: PyObject,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        unsafe {
            self.insert_unsorted(key, value, ttl);
        }

        if self.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(self.order);
        }

        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &HashablePyObject) -> Option<(f32, &PyObject)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, |(x, _)| x.key() == key) {
            Some(bucket) => {
                let (x, val) = unsafe { bucket.as_ref() };

                if x.expired() {
                    None
                } else {
                    Some((x.remaining().unwrap_or(0.0), val))
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(VTTLKey, PyObject)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, |(x, _)| x.key() == key) {
            Some(bucket) => {
                // override key variable
                let (key, _) = unsafe { bucket.as_ref() };

                #[cfg(debug_assertions)]
                let index = self
                    .order
                    .iter()
                    .position(|x| x.key() == key.key())
                    .unwrap();

                #[cfg(not(debug_assertions))]
                let index = unsafe {
                    self.order
                        .iter()
                        .position(|x| x.key() == key.key())
                        .unwrap_unchecked()
                };

                self.order.swap_remove(index);

                let (val, _) = unsafe { self.table.remove(bucket) };
                if val.0.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn order_ref(&self) -> &Vec<VTTLKey> {
        &self.order
    }

    #[inline]
    pub fn order_mut(&mut self) -> &mut Vec<VTTLKey> {
        &mut self.order
    }

    #[inline]
    pub fn contains_key(&self, key: &HashablePyObject) -> bool {
        if self.table.is_empty() {
            return false;
        }

        self.table
            .find(key.hash, |(x, _)| x.key() == key)
            .filter(|bucket| {
                let (k, _) = unsafe { bucket.as_ref() };
                !k.expired()
            })
            .is_some()
    }

    #[inline]
    fn extend_from_dict(
        &mut self,
        dict: &Bound<'_, pyo3::types::PyDict>,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        for (key, value) in dict.iter() {
            let hashable = HashablePyObject::try_from_bound(key)?;
            unsafe {
                self.insert_unsorted(hashable, value.unbind(), ttl);
            }
        }

        if self.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(self.order);
        }

        Ok(())
    }

    #[inline]
    fn extend_from_iter(
        &mut self,
        obj: &pyo3::Bound<'_, PyAny>,
        ttl: Option<f32>,
        py: Python<'_>,
    ) -> PyResult<()> {
        for pair in obj.iter()? {
            let (key, value): (Py<PyAny>, Py<PyAny>) = pair?.extract()?;
            let hashable = HashablePyObject::try_from_pyobject(key, py)?;
            unsafe {
                self.insert_unsorted(hashable, value, ttl);
            }
        }

        if self.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(self.order);
        }

        Ok(())
    }

    pub fn update(&mut self, py: Python<'_>, iterable: PyObject, ttl: Option<f32>) -> PyResult<()> {
        if unsafe { pyo3::ffi::PyDict_Check(iterable.as_ptr()) == 1 } {
            let dict = iterable.downcast_bound::<pyo3::types::PyDict>(py)?;
            self.extend_from_dict(dict, ttl)?;
        } else {
            self.extend_from_iter(iterable.bind(py), ttl, py)?;
        }

        Ok(())
    }
}

impl AsRef<RawTable<(VTTLKey, PyObject)>> for RawVTTLCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(VTTLKey, PyObject)> {
        &self.table
    }
}

impl AsMut<RawTable<(VTTLKey, PyObject)>> for RawVTTLCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(VTTLKey, PyObject)> {
        &mut self.table
    }
}

impl crate::basic::PickleMethods for RawVTTLCache {
    unsafe fn dumps(&self) -> *mut pyo3::ffi::PyObject {
        let dict = pyo3::ffi::PyDict_New();

        for pair in self.table.iter() {
            let (key, val) = pair.as_ref();

            // dict[(key, float)] = val
            //      ------------
            //         object
            //
            // object may not tuple
            match key.timestamp() {
                Some(f) => {
                    let key_tuple = pyo3::ffi::PyTuple_New(2);
                    let timestamp = pyo3::ffi::PyFloat_FromDouble(f);

                    pyo3::ffi::PyTuple_SetItem(key_tuple, 0, key.key().object.as_ptr());
                    pyo3::ffi::PyTuple_SetItem(key_tuple, 1, timestamp);

                    pyo3::ffi::PyDict_SetItem(dict, key_tuple, val.as_ptr());
                    pyo3::ffi::Py_XDECREF(key_tuple);
                }
                None => {
                    pyo3::ffi::PyDict_SetItem(dict, key.key().object.as_ptr(), val.as_ptr());
                }
            }
        }

        let order = pyo3::ffi::PyTuple_New(self.order.len() as isize);
        for (index, key) in self.order.iter().enumerate() {
            match key.timestamp() {
                Some(f) => {
                    let key_tuple = pyo3::ffi::PyTuple_New(2);
                    let timestamp = pyo3::ffi::PyFloat_FromDouble(f);
                    pyo3::ffi::PyTuple_SetItem(key_tuple, 0, key.key().object.as_ptr());
                    pyo3::ffi::PyTuple_SetItem(key_tuple, 1, timestamp);

                    pyo3::ffi::PyTuple_SetItem(order, index as isize, key_tuple);
                }
                None => {
                    pyo3::ffi::PyTuple_SetItem(order, index as isize, key.key().object.as_ptr());
                }
            }
        }

        let maxsize = pyo3::ffi::PyLong_FromSize_t(self.maxsize.get());
        let capacity = pyo3::ffi::PyLong_FromSize_t(self.table.capacity());

        let tuple = pyo3::ffi::PyTuple_New(Self::PICKLE_TUPLE_SIZE);
        pyo3::ffi::PyTuple_SetItem(tuple, 0, maxsize);
        pyo3::ffi::PyTuple_SetItem(tuple, 1, dict);
        pyo3::ffi::PyTuple_SetItem(tuple, 2, capacity);
        pyo3::ffi::PyTuple_SetItem(tuple, 3, order);

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

        let mut new = Self::new(maxsize, capacity)?;

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
            let key_as_ptr = key.as_ptr();

            if pyo3::ffi::PyTuple_CheckExact(key_as_ptr) == 1 {
                if pyo3::ffi::PyTuple_Size(key_as_ptr) != 2 {
                    return Err(create_pyerr!(
                        pyo3::exceptions::PyTypeError,
                        "a value in dictionary that's tuple, but its size isn't equal 2"
                    ));
                }

                let key_object = pyo3::ffi::PyTuple_GetItem(key_as_ptr, 0);
                let timestamp_object = pyo3::ffi::PyTuple_GetItem(key_as_ptr, 1);
                let timestamp = pyo3::ffi::PyFloat_AsDouble(timestamp_object);

                let hashable = HashablePyObject::try_from_pyobject(
                    PyObject::from_borrowed_ptr(py, key_object),
                    py,
                )
                .unwrap_unchecked();
                let vttlkey = VTTLKey::from_timestamp(hashable, Some(timestamp));

                new.insert_unchecked(vttlkey, value.unbind());
            } else {
                let hashable = HashablePyObject::try_from_bound(key).unwrap_unchecked();
                let vttlkey = VTTLKey::from_timestamp(hashable, None);

                new.insert_unchecked(vttlkey, value.unbind());
            }
        }

        if new.order.try_reserve(tuple_length as usize).is_err() {
            return Err(create_pyerr!(pyo3::exceptions::PyMemoryError));
        }

        for k in 0..tuple_length {
            let key_as_ptr = pyo3::ffi::PyTuple_GetItem(order, k);

            if pyo3::ffi::PyTuple_CheckExact(key_as_ptr) == 1 {
                if pyo3::ffi::PyTuple_Size(key_as_ptr) != 2 {
                    return Err(create_pyerr!(
                        pyo3::exceptions::PyTypeError,
                        "a value in dictionary that's tuple, but its size isn't equal 2"
                    ));
                }

                let key_object = pyo3::ffi::PyTuple_GetItem(key_as_ptr, 0);
                let timestamp_object = pyo3::ffi::PyTuple_GetItem(key_as_ptr, 1);
                let timestamp = pyo3::ffi::PyFloat_AsDouble(timestamp_object);

                let hashable = HashablePyObject::try_from_pyobject(
                    PyObject::from_borrowed_ptr(py, key_object),
                    py,
                )?;
                let vttlkey = VTTLKey::from_timestamp(hashable, Some(timestamp));

                new.order.push(vttlkey);
            } else {
                let hashable = HashablePyObject::try_from_pyobject(
                    PyObject::from_borrowed_ptr(py, key_as_ptr),
                    py,
                )?;
                let vttlkey = VTTLKey::from_timestamp(hashable, None);

                new.order.push(vttlkey);
            }
        }

        if new.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(new.order);
        }

        new.expire();

        *self = new;

        Ok(())
    }
}
