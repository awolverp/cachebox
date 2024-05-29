mod raw;

use crate::{basic::HashablePyObject, create_pyerr};
use parking_lot::RwLock;
use pyo3::prelude::*;

pub use self::raw::RawCache;

#[pyclass(extends=crate::basic::BaseCacheImpl, subclass, module="cachebox._cachebox")]
pub struct Cache {
    table: RwLock<RawCache>,
}

#[pymethods]
impl Cache {
    #[new]
    #[pyo3(
        signature=(maxsize, iterable=None, *, capacity=0),
        text_signature="(maxsize, iterable=None, *, capacity=...)"
    )]
    pub fn new(
        py: Python<'_>,
        maxsize: usize,
        iterable: Option<PyObject>,
        capacity: usize,
    ) -> PyResult<PyClassInitializer<Cache>> {
        let slf = Self {
            table: RwLock::new(RawCache::new(maxsize, capacity)?),
        };

        if let Some(x) = iterable {
            slf.update(py, x)?;
        }

        Ok(PyClassInitializer::from(super::basic::BaseCacheImpl).add_subclass(slf))
    }

    #[inline]
    #[getter]
    pub fn maxsize(&self) -> usize {
        self.table.read().maxsize.get()
    }

    #[inline]
    pub fn __len__(&self) -> usize {
        self.table.read().as_ref().len()
    }

    #[inline]
    pub fn __sizeof__(&self) -> usize {
        let cap = self.table.read().as_ref().capacity();

        // capacity * sizeof(HashablePyObject) + capacity * sizeof(PyObject)
        cap * (super::basic::PYOBJECT_MEM_SIZE + 8) + cap * super::basic::PYOBJECT_MEM_SIZE
    }

    #[inline]
    pub fn __bool__(&self) -> bool {
        !self.table.read().as_ref().is_empty()
    }

    #[inline]
    pub fn __setitem__(&self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        lock.insert(hashable, value)
    }

    #[pyo3(text_signature = "(key, value)")]
    #[inline]
    pub fn insert(&self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        self.__setitem__(py, key, value)
    }

    #[inline]
    pub fn __getitem__(&self, py: Python<'_>, key: PyObject) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();

        match lock.get(&hashable) {
            Some(x) => Ok(x),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

    #[inline]
    #[pyo3(
        signature=(key, default=None, /),
        text_signature="(key, default=None, /)"
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
            Some(x) => Ok(x),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    #[inline]
    pub fn __delitem__(&self, py: Python<'_>, key: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some(_) => Ok(()),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

    #[inline]
    pub fn __contains__(&self, py: Python<'_>, key: PyObject) -> PyResult<bool> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();
        Ok(lock.contains_key(&hashable))
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.read().as_ref().capacity()
    }

    #[inline]
    #[pyo3(signature=(*, reuse=false), text_signature="(*, reuse=False)")]
    pub fn clear(&self, reuse: bool) {
        let mut lock = self.table.write();
        let tb = lock.as_mut();
        tb.clear();

        if !reuse {
            tb.shrink_to(0, |(x, _)| x.hash)
        }
    }

    #[inline]
    #[pyo3(signature=(key, default=None, /), text_signature="(key, default=None, /)")]
    pub fn pop(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.remove(&hashable) {
            Some(x) => Ok(x),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    #[inline]
    #[pyo3(signature=(key, default=None, /), text_signature="(key, default=None, /)")]
    pub fn setdefault(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();

        if let Some(x) = lock.get(&hashable) {
            return Ok(x);
        }

        let default_val = default.unwrap_or_else(|| py.None());

        lock.insert(hashable, default_val.clone())?;
        Ok(default_val)
    }

    #[inline]
    pub fn popitem(&self) -> PyResult<()> {
        Err(create_pyerr!(pyo3::exceptions::PyNotImplementedError))
    }

    #[allow(unused_variables)]
    #[inline]
    pub fn drain(&self, n: usize) -> PyResult<()> {
        Err(create_pyerr!(pyo3::exceptions::PyNotImplementedError))
    }

    #[inline]
    fn update(&self, py: Python<'_>, iterable: PyObject) -> PyResult<()> {
        let obj = iterable.bind_borrowed(py);

        if obj.is_instance_of::<pyo3::types::PyDict>() {
            let dict = obj.downcast::<pyo3::types::PyDict>()?;

            let mut lock = self.table.write();
            lock.extend_from_dict(dict)?;
        } else {
            let mut lock = self.table.write();
            lock.extend_from_iter(obj, py)?;
        }

        Ok(())
    }

    #[inline]
    pub fn shrink_to_fit(&self) {
        self.table.write().as_mut().shrink_to(0, |(x, _)| x.hash);
    }

    pub fn items(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::items_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::items_iterator {
            safeiter: crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
        };

        Py::new(py, iter)
    }

    pub fn __iter__(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::keys_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::keys_iterator {
            safeiter: crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
        };

        Py::new(py, iter)
    }

    pub fn keys(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::keys_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::keys_iterator {
            safeiter: crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
        };

        Py::new(py, iter)
    }

    pub fn values(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::values_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::values_iterator {
            safeiter: crate::basic::iter::SafeRawIter::new(slf.as_ptr(), len, iter),
        };

        Py::new(py, iter)
    }

    #[inline]
    pub fn __str__(&self) -> String {
        let lock = self.table.read();
        let tb = lock.as_ref();
        format!(
            "Cache({} / {}, capacity={})",
            tb.len(),
            lock.maxsize.get(),
            tb.capacity()
        )
    }

    fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        for value in unsafe { self.table.read().as_ref().iter() } {
            let (key, value) = unsafe { value.as_ref() };
            visit.call(&key.object)?;
            visit.call(value)?;
        }
        Ok(())
    }

    fn __clear__(&self) {
        let mut t = self.table.write();
        t.as_mut().clear();
    }
}
