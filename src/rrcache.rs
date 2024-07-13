use crate::basic::HashablePyObject;
use crate::cache::RawCache;
use crate::{create_pyerr, make_eq_func, make_hasher_func};
use parking_lot::RwLock;
use pyo3::prelude::*;

/// RRCache implementation - Random Replacement policy (thread-safe).
///
/// In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.
#[pyclass(mapping, extends=crate::basic::BaseCacheImpl, subclass, module="cachebox._cachebox")]
pub struct RRCache {
    table: RwLock<RawCache>,
}

macro_rules! rr_alg_popitem {
    ($tb:expr) => {{
        if $tb.is_empty() {
            Err(create_pyerr!(pyo3::exceptions::PyKeyError))
        } else {
            let rnd = fastrand::usize(0..$tb.len());

            #[cfg(debug_assertions)]
            let bucket = unsafe { $tb.iter().nth(rnd).unwrap() };

            #[cfg(not(debug_assertions))]
            let bucket = unsafe { $tb.iter().nth(rnd).unwrap_unchecked() };

            let (h, _) = unsafe { $tb.remove(bucket) };
            Ok((h.0.object, h.1))
        }
    }};
}

fn rr_alg_insert(
    lock: &mut parking_lot::lock_api::RwLockWriteGuard<'_, parking_lot::RawRwLock, RawCache>,
    hashable: HashablePyObject,
    value: PyObject,
) -> PyResult<()> {
    if lock.as_ref().len() >= lock.maxsize.get() {
        let tb = lock.as_mut();
        if tb.find(hashable.hash, make_eq_func!(hashable)).is_none() {
            rr_alg_popitem!(tb)?;
        }
    }

    unsafe {
        lock.insert_unchecked(hashable, value);
    }

    Ok(())
}

fn rr_alg_update(
    lock: &mut parking_lot::lock_api::RwLockWriteGuard<'_, parking_lot::RawRwLock, RawCache>,
    iterable: PyObject,
    py: Python<'_>,
) -> PyResult<()> {
    if unsafe { pyo3::ffi::PyDict_Check(iterable.as_ptr()) == 1 } {
        let dict = iterable.downcast_bound::<pyo3::types::PyDict>(py)?;

        for (key, value) in dict.iter() {
            let hashable = HashablePyObject::try_from_bound(key)?;
            rr_alg_insert(lock, hashable, value.unbind())?;
        }
    } else {
        let obj = iterable.bind(py);

        for pair in obj.iter()? {
            let (key, value): (Py<PyAny>, Py<PyAny>) = pair?.extract()?;

            let hashable = HashablePyObject::try_from_pyobject(key, py)?;
            rr_alg_insert(lock, hashable, value)?;
        }
    }

    Ok(())
}

#[pymethods]
impl RRCache {
    #[new]
    #[pyo3(signature=(maxsize, iterable=None, *, capacity=0))]
    pub fn new(
        py: Python<'_>,
        maxsize: usize,
        iterable: Option<PyObject>,
        capacity: usize,
    ) -> PyResult<PyClassInitializer<RRCache>> {
        let slf = Self {
            table: RwLock::new(RawCache::new(maxsize, capacity)?),
        };

        if let Some(x) = iterable {
            let mut lock = slf.table.write();
            rr_alg_update(&mut lock, x, py)?;
        }

        Ok(PyClassInitializer::from(super::basic::BaseCacheImpl).add_subclass(slf))
    }

    #[getter]
    pub fn maxsize(&self) -> usize {
        self.table.read().maxsize.get()
    }

    pub fn is_full(&self) -> bool {
        let lock = self.table.read();
        return lock.as_ref().len() == lock.maxsize.get();
    }

    pub fn is_empty(&self) -> bool {
        let lock = self.table.read();
        return lock.as_ref().len() == 0;
    }

    pub fn __len__(&self) -> usize {
        self.table.read().as_ref().len()
    }

    pub fn __sizeof__(&self) -> usize {
        let cap = self.table.read().as_ref().capacity();

        // sizeof(self) + capacity * (sizeof(HashablePyObject) + sizeof(PyObject))
        core::mem::size_of::<Self>()
            + cap * (super::basic::PYOBJECT_MEM_SIZE + super::basic::HASHABLE_PYOBJECT_MEM_SIZE)
    }

    pub fn __bool__(&self) -> bool {
        !self.table.read().as_ref().is_empty()
    }

    pub fn __setitem__(&mut self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let mut lock = self.table.write();
        rr_alg_insert(&mut lock, hashable, value)
    }

    #[pyo3(text_signature = "(key, value)")]
    pub fn insert(&mut self, py: Python<'_>, key: PyObject, value: PyObject) -> PyResult<()> {
        self.__setitem__(py, key, value)
    }

    pub fn __getitem__(&self, py: Python<'_>, key: PyObject) -> PyResult<PyObject> {
        let hashable = HashablePyObject::try_from_pyobject(key, py)?;
        let lock = self.table.read();

        match lock.get(&hashable) {
            Some(x) => Ok(x.clone()),
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError, hashable.object)),
        }
    }

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

    #[pyo3(signature=(*, reuse=false), text_signature="(*, reuse=False)")]
    pub fn clear(&mut self, reuse: bool) {
        let mut lock = self.table.write();
        let tb = lock.as_mut();
        tb.clear();

        if !reuse {
            tb.shrink_to(0, make_hasher_func!());
        }
    }

    #[pyo3(signature=(key, default=None, /), text_signature="(key, default=None, /)")]
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

    #[pyo3(signature=(key, default=None, /), text_signature="(key, default=None, /)")]
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
        let tb = lock.as_mut();
        rr_alg_popitem!(tb)
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut lock = self.table.write();
        let tb = lock.as_mut();

        if n == 0 || tb.is_empty() {
            return 0;
        }

        let mut c = 0usize;
        while c < n {
            if rr_alg_popitem!(tb).is_err() {
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
        rr_alg_update(&mut lock, iterable, py)
    }

    pub fn shrink_to_fit(&self) {
        self.table
            .write()
            .as_mut()
            .shrink_to(0, make_hasher_func!());
    }

    pub fn items(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::tuple_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::tuple_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
        );

        Py::new(py, iter)
    }

    pub fn __iter__(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::object_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn keys(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::object_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::object_ptr_iterator::new(
            crate::basic::iter::SafeRawHashMapIter::new(slf.as_ptr(), capacity, len, iter),
            0,
        );

        Py::new(py, iter)
    }

    pub fn values(
        slf: PyRef<'_, Self>,
        py: Python<'_>,
    ) -> PyResult<Py<crate::basic::iter::object_ptr_iterator>> {
        let lock = slf.table.read();
        let len = lock.as_ref().len();
        let capacity = lock.as_ref().capacity();
        let iter = unsafe { lock.as_ref().iter() };

        let iter = crate::basic::iter::object_ptr_iterator::new(
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
            "RRCache({} / {}, capacity={})",
            tb.len(),
            lock.maxsize.get(),
            tb.capacity()
        )
    }

    pub fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        for value in unsafe { self.table.read().as_ref().iter() } {
            let (key, value) = unsafe { value.as_ref() };
            visit.call(&key.object)?;
            visit.call(value)?;
        }
        Ok(())
    }

    pub fn __clear__(&mut self) {
        let mut t = self.table.write();
        t.as_mut().clear();
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
        let tuple = crate::pickle_check_state!(py, state, RawCache::PICKLE_TUPLE_SIZE)?;

        let mut lock = self.table.write();
        unsafe { lock.loads(tuple, py) }
    }
}
