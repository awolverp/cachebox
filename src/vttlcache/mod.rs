mod raw;

use self::raw::{RawVTTLCache, VTTLKey};
use crate::basic::iter::SafeRawIter;
use crate::basic::HashablePyObject;
use crate::create_pyerr;
use parking_lot::RwLock;
use pyo3::prelude::*;

#[pyclass(mapping, extends=crate::basic::BaseCacheImpl, subclass, module="cachebox._cachebox")]
pub struct VTTLCache {
    table: RwLock<RawVTTLCache>,
}

#[pymethods]
impl VTTLCache {
    #[new]
    #[pyo3(signature=(maxsize, iterable=None, ttl=None, *, capacity=0))]
    pub fn new(
        py: Python<'_>,
        maxsize: usize,
        iterable: Option<PyObject>,
        ttl: Option<f32>,
        capacity: usize,
    ) -> PyResult<PyClassInitializer<VTTLCache>> {
        let slf = Self {
            table: RwLock::new(RawVTTLCache::new(maxsize, capacity)?),
        };

        if let Some(x) = iterable {
            slf.update(py, x, ttl)?;
        }

        Ok(PyClassInitializer::from(super::basic::BaseCacheImpl).add_subclass(slf))
    }

    #[getter]
    pub fn maxsize(&self) -> usize {
        self.table.read().maxsize.get()
    }

    pub fn is_full(&self) -> bool {
        let mut lock = self.table.write();
        lock.expire();
        lock.as_ref().len() == lock.maxsize.get()
    }

    pub fn is_empty(&self) -> bool {
        let mut lock = self.table.write();
        lock.expire();
        lock.as_ref().len() == 0
    }

    pub fn __len__(&self) -> usize {
        let mut lock = self.table.write();
        lock.expire();
        lock.as_ref().len()
    }

    pub fn __sizeof__(&self) -> usize {
        let lock = self.table.read();
        let cap = lock.as_ref().capacity();
        let o_cap = lock.order_ref().capacity();

        // capacity * sizeof(TTLKey) + capacity * sizeof(HashablePyObject) + order_capacity * sizeof(HashablePyObject)
        core::mem::size_of::<Self>()
            + cap * (VTTLKey::SIZE + super::basic::PYOBJECT_MEM_SIZE)
            + o_cap * VTTLKey::SIZE
    }

    pub fn __bool__(&self) -> bool {
        let mut lock = self.table.write();
        lock.expire();
        !lock.as_ref().is_empty()
    }

    pub fn __setitem__(&self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        lock.expire();
        lock.insert(hashable, value, None)
    }

    #[pyo3(text_signature = "(key, value)")]
    pub fn insert(
        &self,
        py: Python<'_>,
        key: PyObject,
        value: PyObject,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        lock.expire();
        lock.insert(hashable, value, ttl)
    }

    pub fn __getitem__(&self, py: Python<'_>, key: PyObject) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();

        match lock.get(&hashable) {
            Some((_, x)) => Ok(x.clone()),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

    #[pyo3(
        signature=(key, default=None),
        text_signature="(key, default=None)"
    )]
    pub fn get(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();
        match lock.get(&hashable) {
            Some((_, x)) => Ok(x.clone()),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    pub fn __delitem__(&self, py: Python<'_>, key: PyObject) -> PyResult<()> {
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
    pub fn clear(&self, reuse: bool) {
        let mut lock = self.table.write();
        let tb = lock.as_mut();
        tb.clear();

        if !reuse {
            tb.shrink_to(0, |(x, _)| x.key().hash);
        }

        let order = lock.order_mut();
        order.clear();
        if !reuse {
            order.shrink_to_fit();
        }
    }

    #[pyo3(signature=(key, default=None), text_signature="(key, default=None)")]
    pub fn pop(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some(x) => Ok(x.1),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    #[pyo3(signature=(key, default=None, ttl=None))]
    pub fn setdefault(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
        ttl: Option<f32>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();

        if let Some(x) = lock.get(&hashable) {
            return Ok(x.1.clone());
        }

        let default_val = default.unwrap_or_else(|| py.None());

        lock.insert(hashable, default_val.clone(), ttl)?;
        Ok(default_val)
    }

    pub fn popitem(&self) -> PyResult<(PyObject, PyObject)> {
        let mut lock = self.table.write();
        lock.expire();
        let (k, v) = lock.popitem()?;
        Ok((k.into_key().object, v))
    }

    pub fn drain(&self, n: usize) -> usize {
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

    #[pyo3(signature=(iterable, ttl=None))]
    fn update(&self, py: Python<'_>, iterable: PyObject, ttl: Option<f32>) -> PyResult<()> {
        let obj = iterable.bind_borrowed(py);

        if obj.is_instance_of::<pyo3::types::PyDict>() {
            let dict = obj.downcast::<pyo3::types::PyDict>()?;
            let mut lock = self.table.write();
            lock.expire();
            lock.extend_from_dict(dict, ttl)?;
        } else {
            let mut lock = self.table.write();
            lock.expire();
            lock.extend_from_iter(obj, ttl, py)?;
        }

        Ok(())
    }

    pub fn shrink_to_fit(&self) {
        let mut lock = self.table.write();
        lock.as_mut().shrink_to(0, |(x, _)| x.key().hash);
        lock.order_mut().shrink_to_fit();
    }

    pub fn items(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<vttl_tuple_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();
        
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = vttl_tuple_ptr_iterator::new(crate::basic::iter::SafeRawIter::new(
            slf.as_ptr(),
            len,
            iter,
        ));

        Py::new(py, iter)
    }

    pub fn __iter__(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<vttl_object_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();
        
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = vttl_object_ptr_iterator::new(
            crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn keys(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<vttl_object_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();
        
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = vttl_object_ptr_iterator::new(
            crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn values(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<vttl_object_ptr_iterator>> {
        let mut lock = slf.table.write();
        lock.expire();
        
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = vttl_object_ptr_iterator::new(
            crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
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
                let (k1, v1) = x.as_ref();
                t2.find(k1.key().hash, |(vttlk, _)| vttlk.key() == k1.key())
                    .map_or(false, |y| {
                        let (k2, v2) = y.as_ref();

                        match (k1.expired(), k2.expired()) {
                            (true, true) => {
                                // ignore expired cases
                                return true;
                            }
                            (false, false) => (),
                            _ => return false,
                        }

                        let res = pyo3::ffi::PyObject_RichCompareBool(
                            v1.as_ptr(),
                            v2.as_ptr(),
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
            "VTTLCache({} / {}, capacity={})",
            tb.len(),
            lock.maxsize.get(),
            tb.capacity()
        )
    }

    pub fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        let lock = self.table.read();
        for value in unsafe { lock.as_ref().iter() } {
            let (key, value) = unsafe { value.as_ref() };
            visit.call(&key.key().object)?;
            visit.call(value)?;
        }
        for value in lock.order_ref().iter() {
            visit.call(&value.key().object)?;
        }

        Ok(())
    }

    pub fn __clear__(&self) {
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
            Some((ttl, x)) => Ok((x.clone(), ttl)),
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    #[pyo3(signature=(key, default=None), text_signature="(key, default=None)")]
    pub fn pop_with_expire(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<(PyObject, f32)> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some((ttl, t)) => Ok((t, ttl.remaining().unwrap_or(0.0))),
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    pub fn popitem_with_expire(&self) -> PyResult<(PyObject, PyObject, f32)> {
        let mut lock = self.table.write();
        lock.expire();
        let (k, v) = lock.popitem()?;
        let d = k.remaining().unwrap_or(0.0);
        Ok((k.into_key().object, v, d))
    }
}

#[allow(non_camel_case_types)]
#[pyclass(module = "cachebox._cachebox")]
pub struct vttl_tuple_ptr_iterator {
    iter: SafeRawIter<(VTTLKey, PyObject)>,
}

impl vttl_tuple_ptr_iterator {
    pub fn new(iter: SafeRawIter<(VTTLKey, PyObject)>) -> Self {
        Self { iter }
    }
}

#[pymethods]
impl vttl_tuple_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<(PyObject, PyObject)> {
        let mut item = slf.iter.next()?;
        while item.0.expired() {
            item = slf.iter.next()?;
        }

        Ok((item.0.key().object.clone(), item.1.clone()))
    }
}

#[allow(non_camel_case_types)]
#[pyclass(module = "cachebox._cachebox")]
pub struct vttl_object_ptr_iterator {
    iter: SafeRawIter<(VTTLKey, PyObject)>,
    index: u8,
}

impl vttl_object_ptr_iterator {
    pub fn new(iter: SafeRawIter<(VTTLKey, PyObject)>, index: u8) -> Self {
        Self { iter, index }
    }
}

#[pymethods]
impl vttl_object_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyObject> {
        let index = slf.index;
        let mut item = slf.iter.next()?;
        while item.0.expired() {
            item = slf.iter.next()?;
        }

        if index == 0 {
            Ok(item.0.key().object.clone())
        } else if index == 1 {
            Ok(item.1.clone())
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
