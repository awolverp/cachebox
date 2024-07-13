mod raw;

use self::raw::RawLFUCache;
use crate::basic::iter::SafeRawHashMapIter;
use crate::basic::HashablePyObject;
use crate::create_pyerr;
use parking_lot::RwLock;
use pyo3::prelude::*;

/// LFU Cache implementation - Least frequantly used policy (thread-safe).
///
/// In simple terms, the LFU cache will remove the element in the cache that has been accessed the least,
/// regardless of time
#[pyclass(mapping, extends=crate::basic::BaseCacheImpl, subclass, module="cachebox._cachebox")]
pub struct LFUCache {
    table: RwLock<RawLFUCache>,
}

#[pymethods]
impl LFUCache {
    #[new]
    #[pyo3(signature=(maxsize, iterable=None, *, capacity=0))]
    pub fn new(
        py: Python<'_>,
        maxsize: usize,
        iterable: Option<PyObject>,
        capacity: usize,
    ) -> PyResult<PyClassInitializer<LFUCache>> {
        let mut table = RawLFUCache::new(maxsize, capacity)?;
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

    pub fn is_full(&self) -> bool {
        let lock = self.table.read();
        lock.as_ref().len() == lock.maxsize.get()
    }

    pub fn is_empty(&self) -> bool {
        let lock = self.table.read();
        lock.as_ref().len() == 0
    }

    pub fn __len__(&self) -> usize {
        self.table.read().as_ref().len()
    }

    pub fn __sizeof__(&self) -> usize {
        let cap = self.table.read().as_ref().capacity();

        // sizeof(self) + capacity * (sizeof(HashablePyObject) + sizeof(PyObject))
        core::mem::size_of::<Self>()
            + cap
                * (super::basic::PYOBJECT_MEM_SIZE
                    + super::basic::HASHABLE_PYOBJECT_MEM_SIZE
                    + core::mem::size_of::<usize>())
    }

    pub fn __bool__(&self) -> bool {
        !self.table.read().as_ref().is_empty()
    }

    pub fn __setitem__(&mut self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        lock.insert(hashable, value)
    }

    #[pyo3(text_signature = "(key, value)")]
    pub fn insert(&mut self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        self.__setitem__(py, key, value)
    }

    pub fn __getitem__(&mut self, py: Python<'_>, key: PyObject) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();

        match lock.get(&hashable) {
            Some(x) => Ok(x.clone()),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn get(
        &mut self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        match lock.get(&hashable) {
            Some(x) => Ok(x.clone()),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn peek(
        &self,
        py: Python<'_>,
        key: PyObject,
        default: Option<PyObject>,
    ) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();
        match lock.peek(&hashable) {
            Some(x) => Ok(x.clone()),
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
            tb.shrink_to(0, |(x, _, _)| x.hash);
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
            Some(x) => Ok(x.1),
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
            return Ok(x.clone());
        }

        let default_val = default.unwrap_or_else(|| py.None());

        lock.insert(hashable, default_val.clone())?;
        Ok(default_val)
    }

    pub fn popitem(&mut self) -> PyResult<(PyObject, PyObject)> {
        let mut lock = self.table.write();
        let (k, v) = lock.popitem()?;
        Ok((k.object, v))
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut lock = self.table.write();

        if n == 0 || lock.as_ref().is_empty() {
            return 0;
        }

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
        lock.as_mut().shrink_to(0, |(x, _, _)| x.hash);
    }

    pub fn items(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<lfu_tuple_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = lfu_tuple_ptr_iterator::new(crate::basic::iter::SafeRawHashMapIter::new(
            slf.as_ptr(),
            capacity,
            len,
            iter,
        ));

        Py::new(py, iter)
    }

    pub fn __iter__(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<lfu_object_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = lfu_object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn keys(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<lfu_object_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = lfu_object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn values(slf: PyRef<'_, Self>, py: Python<'_>) -> PyResult<Py<lfu_object_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = lfu_object_ptr_iterator::new(
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
                let (k, v1, _) = x.as_ref();
                t2.find(k.hash, |(x, _, _)| x.eq(k)).map_or(false, |y| {
                    let (_, v2, _) = y.as_ref();

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
            "LFUCache({} / {}, capacity={})",
            tb.len(),
            lock.maxsize.get(),
            tb.capacity()
        )
    }

    pub fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        for value in unsafe { self.table.read().as_ref().iter() } {
            let (key, value, _) = unsafe { value.as_ref() };
            visit.call(&key.object)?;
            visit.call(value)?;
        }
        Ok(())
    }

    pub fn __clear__(&mut self) {
        let mut t = self.table.write();
        t.as_mut().clear();
    }

    #[pyo3(signature=(n=0))]
    pub fn least_frequently_used(&self, n: usize) -> Option<PyObject> {
        let lock = self.table.read();
        lock.least_frequently_used(n).map(|h| h.object.clone())
    }

    pub fn __getstate__(&self, py: Python<'_>) -> PyObject {
        use crate::basic::PickleMethods;

        let lock = self.table.read();

        unsafe {
            let state = lock.dumps();
            Py::from_owned_ptr(py, state)
        }
    }

    pub fn __getnewargs__(&self) -> (usize,) {
        (0,)
    }

    pub fn __setstate__(&mut self, py: Python<'_>, state: PyObject) -> PyResult<()> {
        use crate::basic::PickleMethods;
        let tuple = crate::pickle_check_state!(py, state, RawLFUCache::PICKLE_TUPLE_SIZE)?;

        let mut lock = self.table.write();
        unsafe { lock.loads(tuple, py) }
    }
}

#[allow(non_camel_case_types)]
#[pyclass(module = "cachebox._cachebox")]
pub struct lfu_tuple_ptr_iterator {
    iter: SafeRawHashMapIter<(HashablePyObject, PyObject, usize)>,
}

impl lfu_tuple_ptr_iterator {
    pub fn new(iter: SafeRawHashMapIter<(HashablePyObject, PyObject, usize)>) -> Self {
        Self { iter }
    }
}

#[pymethods]
impl lfu_tuple_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<(PyObject, PyObject)> {
        let (k, v, _) = slf.iter.next(py)?;
        Ok((k.object.clone(), v.clone()))
    }
}

#[allow(non_camel_case_types)]
#[pyclass(module = "cachebox._cachebox")]
pub struct lfu_object_ptr_iterator {
    iter: SafeRawHashMapIter<(HashablePyObject, PyObject, usize)>,
    index: u8,
}

impl lfu_object_ptr_iterator {
    pub fn new(iter: SafeRawHashMapIter<(HashablePyObject, PyObject, usize)>, index: u8) -> Self {
        Self { iter, index }
    }
}

#[pymethods]
impl lfu_object_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<PyObject> {
        if slf.index == 0 {
            let (k, _, _) = slf.iter.next(py)?;
            Ok(k.object.clone())
        } else if slf.index == 1 {
            let (_, v, _) = slf.iter.next(py)?;
            Ok(v.clone())
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
