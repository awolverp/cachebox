use crate::basic::{HashablePyObject, PickleMethods};
use crate::{create_pyerr, make_eq_func, make_hasher_func, pickle_get_first_objects};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;
use std::collections::VecDeque;

pub struct RawLRUCache {
    table: RawTable<(HashablePyObject, PyObject)>,
    order: VecDeque<HashablePyObject>,
    pub maxsize: NonZeroUsize,
}

macro_rules! vecdeque_move_to_end {
    ($order:expr, $key:expr) => {{
        #[cfg(debug_assertions)]
        let index = $order.iter().position(|x| x.eq($key)).unwrap();

        #[cfg(not(debug_assertions))]
        let index = unsafe { $order.iter().position(|x| x.eq($key)).unwrap_unchecked() };

        let item = unsafe { $order.remove(index).unwrap_unchecked() };
        $order.push_back(item);
    }};
}

impl RawLRUCache {
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
            order: VecDeque::new(),
        })
    }

    #[inline]
    pub fn popitem(&mut self) -> PyResult<(HashablePyObject, PyObject)> {
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
    pub unsafe fn insert_unchecked(&mut self, key: HashablePyObject, value: PyObject) -> bool {
        match self
            .table
            .find_or_find_insert_slot(key.hash, make_eq_func!(key), make_hasher_func!())
        {
            Ok(bucket) => {
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().1 }, value);
                false
            }
            Err(slot) => unsafe {
                self.table.insert_in_slot(key.hash, slot, (key, value));
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

        if unsafe { self.insert_unchecked(key.clone(), value) } {
            self.order.push_back(key);
        } else {
            vecdeque_move_to_end!(self.order, &key);
        }

        Ok(())
    }

    #[inline]
    pub fn get(&mut self, key: &HashablePyObject) -> Option<&PyObject> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                vecdeque_move_to_end!(self.order, key);
                let (_, val) = unsafe { bucket.as_ref() };
                Some(val)
            }
            None => None,
        }
    }

    #[inline]
    pub fn peek(&self, key: &HashablePyObject) -> Option<&PyObject> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                let (_, val) = unsafe { bucket.as_ref() };
                Some(val)
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(HashablePyObject, PyObject)> {
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
                Some(val)
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

    #[inline]
    pub fn least_recently_used(&self) -> Option<&HashablePyObject> {
        self.order.front()
    }

    #[inline]
    pub fn most_recently_used(&self) -> Option<&HashablePyObject> {
        self.order.back()
    }
}

impl AsRef<RawTable<(HashablePyObject, PyObject)>> for RawLRUCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(HashablePyObject, PyObject)> {
        &self.table
    }
}

impl AsMut<RawTable<(HashablePyObject, PyObject)>> for RawLRUCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(HashablePyObject, PyObject)> {
        &mut self.table
    }
}

impl PickleMethods for RawLRUCache {
    unsafe fn dumps(&self) -> *mut pyo3::ffi::PyObject {
        let dict = pyo3::ffi::PyDict_New();

        for pair in self.table.iter() {
            let (key, val) = pair.as_ref();
            // SAFETY: we don't need to check error because we sure about key that is hashable
            pyo3::ffi::PyDict_SetItem(dict, key.object.as_ptr(), val.as_ptr());
        }

        let order = pyo3::ffi::PyTuple_New(self.order.len() as isize);
        for (index, key) in self.order.iter().enumerate() {
            pyo3::ffi::PyTuple_SetItem(order, index as isize, key.object.as_ptr());
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
            // SAFETY: key is hashable, so don't worry
            let hashable = HashablePyObject::try_from_bound(key).unwrap_unchecked();
            new.insert_unchecked(hashable, value.unbind());
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

        *self = new;

        Ok(())
    }
}
