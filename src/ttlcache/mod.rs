mod raw;

use self::raw::{RawTTLCache, TTLValue};
use crate::basic::iter::SafeRawHashMapIter;
use crate::basic::HashablePyObject;
use crate::{create_pyerr, make_eq_func, make_hasher_func};
use parking_lot::RwLock;
use pyo3::prelude::*;

/// TTL Cache implementation - Time-To-Live Policy (thread-safe).
///
/// In simple terms, the TTL cache will automatically remove the element in the cache that has expired::
#[pyclass(mapping, extends=crate::basic::BaseCacheImpl, subclass, module="cachebox._cachebox")]
pub struct TTLCache {
    table: RwLock<RawTTLCache>,
}

#[pymethods]
impl TTLCache {
    #[new]
    #[pyo3(signature=(maxsize, ttl, iterable=None, *, capacity=0))]
    pub fn new(
        py: Python<'_>,
        maxsize: usize,
        ttl: f32,
        iterable: Option<PyObject>,
        capacity: usize,
    ) -> PyResult<PyClassInitializer<TTLCache>> {
        let mut table = RawTTLCache::new(maxsize, ttl, capacity)?;
        if let Some(x) = iterable {
            table.update(py, x)?;
        }

        let slf = Self {
            table: RwLock::new(table),
        };

        Ok(PyClassInitializer::from(super::basic::BaseCacheImpl).add_subclass(slf))
    }

    #[getter]
    pub fn maxsize(&self) -> usize {
        self.table.read().maxsize.get()
    }

    #[getter]
    pub fn ttl(&self) -> f32 {
        self.table.read().ttl
    }

    pub fn is_full(&mut self) -> bool {
        let mut lock = self.table.write();
        lock.expire();
        lock.as_ref().len() == lock.maxsize.get()
    }

    pub fn is_empty(&mut self) -> bool {
        let mut lock = self.table.write();
        lock.expire();
        lock.as_ref().len() == 0
    }

    pub fn __len__(&mut self) -> usize {
        let mut lock = self.table.write();
        lock.expire();
        lock.as_ref().len()
    }

    pub fn __sizeof__(&self) -> usize {
        let lock = self.table.read();
        let cap = lock.as_ref().capacity();
        let o_cap = lock.order_ref().capacity();

        // capacity * sizeof(TTLValue) + capacity * sizeof(HashablePyObject) + order_capacity * sizeof(HashablePyObject)
        core::mem::size_of::<Self>()
            + cap * (TTLValue::SIZE + super::basic::HASHABLE_PYOBJECT_MEM_SIZE)
            + o_cap * super::basic::HASHABLE_PYOBJECT_MEM_SIZE
    }

    pub fn __bool__(&mut self) -> bool {
        let mut lock = self.table.write();
        lock.expire();
        !lock.as_ref().is_empty()
    }

    pub fn __setitem__(&mut self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        lock.expire();
        lock.insert(hashable, value)
    }

    #[pyo3(text_signature = "(key, value)")]
    pub fn insert(&mut self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        self.__setitem__(py, key, value)
    }

    pub fn __getitem__(&self, py: Python<'_>, key: PyObject) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();

        match lock.get(&hashable) {
            Some(x) => Ok(x.0.clone()),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn get(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();
        match lock.get(&hashable) {
            Some(x) => Ok(x.0.clone()),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    pub fn __delitem__(&mut self, py: Python<'_>, key: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some(_) => Ok(()),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

    pub fn __contains__(&self, py: Python<'_>, key: PyObject) -> PyResult<bool> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();
        Ok(lock.contains_key(&hashable))
    }

    pub fn capacity(&self) -> usize {
        self.table.read().as_ref().capacity()
    }

    #[pyo3(signature=(*, reuse=false))]
    pub fn clear(&mut self, reuse: bool) {
        let mut lock = self.table.write();
        let tb = lock.as_mut();
        tb.clear();

        if !reuse {
            tb.shrink_to(0, make_hasher_func!());
        }

        let order = lock.order_mut();
        order.clear();
        if !reuse {
            order.shrink_to_fit();
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn pop(
        &mut self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some(x) => Ok(x.1 .0),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn setdefault(
        &mut self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();

        if let Some(x) = lock.get(&hashable) {
            return Ok(x.0.clone());
        }

        let default_val = default.unwrap_or_else(|| py.None());

        lock.insert(hashable, default_val.clone())?;
        Ok(default_val)
    }

    pub fn popitem(&mut self) -> PyResult<(PyObject, PyObject)> {
        let mut lock = self.table.write();
        lock.expire();
        let (k, v) = lock.popitem()?;
        Ok((k.object, v.0))
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut lock = self.table.write();

        if n == 0 || lock.as_ref().is_empty() {
            return 0;
        }

        lock.expire();

        let mut c = 0usize;
        while c < n {
            if lock.popitem().is_err() {
                break;
            }
            c += 1;
        }

        c
    }

    fn update(slf: PyRefMut<'_, Self>, py: Python<'_>, iterable: PyObject) -> PyResult<()> {
        if slf.as_ptr() == iterable.as_ptr() {
            return Ok(());
        }

        let mut lock = slf.table.write();
        lock.update(py, iterable)
    }

    pub fn shrink_to_fit(&mut self) {
        let mut lock = self.table.write();
        lock.as_mut().shrink_to(0, make_hasher_func!());
        lock.order_mut().shrink_to_fit();
    }

    pub fn items(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<ttl_tuple_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();

        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = ttl_tuple_ptr_iterator::new(crate::basic::iter::SafeRawHashMapIter::new(
            slf.as_ptr(),
            capacity,
            len,
            iter,
        ));

        Py::new(py, iter)
    }

    pub fn __iter__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<ttl_object_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();

        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = ttl_object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn keys(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<ttl_object_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();

        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = ttl_object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn values(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<ttl_object_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();

        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = ttl_object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            1,
        );

        Py::new(py, iter)
    }

    pub fn __eq__(&self, other: &Self) -> bool {
        let self_lock = self.table.read();
        let other_lock = other.table.read();

        if self_lock.maxsize != other_lock.maxsize {
            return false;
        }

        let (t1, t2) = (self_lock.as_ref(), other_lock.as_ref());

        if t1.len() != t2.len() {
            return false;
        }

        unsafe {
            t1.iter().all(|x| {
                let (k, v1) = x.as_ref();
                t2.find(k.hash, make_eq_func!(k)).map_or(false, |y| {
                    let (_, v2) = y.as_ref();

                    match (v1.expired(), v2.expired()) {
                        (true, true) => {
                            // ignore expired cases
                            return true;
                        }
                        (false, false) => (),
                        _ => return false,
                    }

                    let res = pyo3::ffi::PyObject_RichCompareBool(
                        v1.0.as_ptr(),
                        v2.0.as_ptr(),
                        pyo3::pyclass::CompareOp::Eq as std::os::raw::c_int,
                    );

                    if res == -1 {
                        pyo3::ffi::PyErr_Clear();
                    }

                    res == 1
                })
            })
        }
    }

    pub fn __ne__(&self, other: &Self) -> bool {
        !self.__eq__(other)
    }

    pub fn __str__(&self) -> String {
        let lock = self.table.read();
        let tb = lock.as_ref();
        format!(
            "TTLCache({} / {}, ttl={}, capacity={})",
            tb.len(),
            lock.maxsize.get(),
            lock.ttl,
            tb.capacity()
        )
    }

    pub fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        let lock = self.table.read();
        for value in unsafe { lock.as_ref().iter() } {
            let (key, value) = unsafe { value.as_ref() };
            visit.call(&key.object)?;
            visit.call(&value.0)?;
        }
        for value in lock.order_ref().iter() {
            visit.call(&value.object)?;
        }

        Ok(())
    }

    pub fn __clear__(&mut self) {
        let mut t = self.table.write();
        t.as_mut().clear();
        t.order_mut().clear();
    }

    #[pyo3(
        signature=(key, default=None),
        text_signature="(key, default=None)"
    )]
    pub fn get_with_expire(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<(PyObject, f32)> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();
        match lock.get(&hashable) {
            Some(x) => Ok((x.0.clone(), x.remaining())),
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    #[pyo3(signature=(key, default=None), text_signature="(key, default=None)")]
    pub fn pop_with_expire(
        &mut self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<(PyObject, f32)> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some((_, t)) => {
                let d = t.remaining();
                Ok((t.0, d))
            }
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    pub fn popitem_with_expire(&mut self) -> PyResult<(PyObject, PyObject, f32)> {
        let mut lock = self.table.write();
        lock.expire();
        let (k, v) = lock.popitem()?;
        let d = v.remaining();
        Ok((k.object, v.0, d))
    }

    #[pyo3(signature=(n=0))]
    pub fn first(&self, n: usize) -> Option<PyObject> {
        let lock = self.table.read();

        let h = {
            if n == 0 {
                lock.first()?
            } else {
                lock.order_ref().get(n)?
            }
        };

        Some(h.object.clone())
    }

    pub fn last(&self) -> Option<PyObject> {
        let lock = self.table.read();
        let h = lock.last()?;
        Some(h.object.clone())
    }

    pub fn __getstate__(&self, py: Python<'_>) -> PyObject {
        use crate::basic::PickleMethods;

        let lock = self.table.read();

        unsafe {
            let state = lock.dumps();
            Py::from_owned_ptr(py, state)
        }
    }

    pub fn __getnewargs__(&self) -> (usize, f32) {
        (0, f32::MAX)
    }

    pub fn __setstate__(&mut self, py: Python<'_>, state: PyObject) -> PyResult<()> {
        use crate::basic::PickleMethods;
        let tuple = crate::pickle_check_state!(py, state, RawTTLCache::PICKLE_TUPLE_SIZE)?;

        let mut lock = self.table.write();
        unsafe { lock.loads(tuple, py) }
    }
}

#[allow(non_camel_case_types)]
#[pyclass(module = "cachebox._cachebox")]
pub struct ttl_tuple_ptr_iterator {
    iter: SafeRawHashMapIter<(HashablePyObject, TTLValue)>,
}

impl ttl_tuple_ptr_iterator {
    pub fn new(iter: SafeRawHashMapIter<(HashablePyObject, TTLValue)>) -> Self {
        Self { iter }
    }
}

#[pymethods]
impl ttl_tuple_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<(PyObject, PyObject)> {
        let mut item = slf.iter.next(py)?;
        while item.1.expired() {
            item = slf.iter.next(py)?;
        }

        Ok((item.0.object.clone(), item.1 .0.clone()))
    }
}

#[allow(non_camel_case_types)]
#[pyclass(module = "cachebox._cachebox")]
pub struct ttl_object_ptr_iterator {
    iter: SafeRawHashMapIter<(HashablePyObject, TTLValue)>,
    index: u8,
}

impl ttl_object_ptr_iterator {
    pub fn new(iter: SafeRawHashMapIter<(HashablePyObject, TTLValue)>, index: u8) -> Self {
        Self { iter, index }
    }
}

#[pymethods]
impl ttl_object_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<PyObject> {
        let index = slf.index;
        let mut item = slf.iter.next(py)?;
        while item.1.expired() {
            item = slf.iter.next(py)?;
        }

        if index == 0 {
            Ok(item.0.object.clone())
        } else if index == 1 {
            Ok(item.1 .0.clone())
        } else {
            #[cfg(debug_assertions)]
            unreachable!("invalid iteration index specified");

            #[cfg(not(debug_assertions))]
            unsafe {
                core::hint::unreachable_unchecked();
            }
        }
    }
}
