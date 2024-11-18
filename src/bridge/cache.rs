//! implement Cache, our simple cache without any algorithms and policies

use crate::hashedkey::HashedKey;
use crate::util::_KeepForIter;

/// A simple cache that has no algorithm; this is only a hashmap.
///
/// [`Cache`] vs `dict`:
/// - it is thread-safe and unordered, while `dict` isn't thread-safe and ordered (Python 3.6+).
/// - it uses very lower memory than `dict`.
/// - it supports useful and new methods for managing memory, while `dict` does not.
/// - it does not support `popitem`, while `dict` does.
/// - You can limit the size of [`Cache`], but you cannot for `dict`.
#[pyo3::pyclass(module="cachebox._cachebox", extends=crate::bridge::baseimpl::BaseCacheImpl, frozen)]
pub struct Cache {
    // Why [`Box`]? We using [`Box`] here so that there's no need for `&mut self`
    // in this struct; so RuntimeError never occurred for using this class in multiple threads.
    raw: Box<crate::mutex::Mutex<crate::internal::NoPolicy>>,
}

#[pyo3::pymethods]
impl Cache {
    /// A simple cache that has no algorithm; this is only a hashmap.
    ///
    /// By maxsize param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
    ///
    /// By iterable param, you can create cache from a dict or an iterable.
    ///
    /// If capacity param is given, cache attempts to allocate a new hash table with at
    /// least enough capacity for inserting the given number of elements without reallocating.
    #[new]
    #[pyo3(signature=(maxsize, iterable=None, *, capacity=0))]
    pub fn __new__(
        py: pyo3::Python<'_>,
        maxsize: usize,
        iterable: Option<pyo3::PyObject>,
        capacity: usize,
    ) -> pyo3::PyResult<(Self, crate::bridge::baseimpl::BaseCacheImpl)> {
        let mut raw = crate::internal::NoPolicy::new(maxsize, capacity)?;
        if iterable.is_some() {
            raw.update(py, unsafe { iterable.unwrap_unchecked() })?;
        }

        let self_ = Self {
            raw: Box::new(crate::mutex::Mutex::new(raw)),
        };
        Ok((self_, crate::bridge::baseimpl::BaseCacheImpl {}))
    }

    /// Returns the cache maxsize
    #[getter]
    pub fn maxsize(&self) -> usize {
        let lock = self.raw.lock();
        lock.maxsize.get()
    }

    /// Returns the number of elements in the table - len(self)
    pub fn __len__(&self) -> usize {
        let lock = self.raw.lock();
        lock.table.len()
    }

    /// Returns allocated memory size - sys.getsizeof(self)
    pub fn __sizeof__(&self) -> usize {
        let lock = self.raw.lock();
        let cap = lock.table.capacity();

        core::mem::size_of::<Self>() + cap * (crate::HASHEDKEY_SIZE + crate::PYOBJECT_SIZE)
    }

    /// Returns true if cache not empty - bool(self)
    pub fn __bool__(&self) -> bool {
        let lock = self.raw.lock();
        !lock.table.is_empty()
    }

    /// Returns true if the cache have the key present - key in self
    pub fn __contains__(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<bool> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let lock = self.raw.lock();
        Ok(lock.contains_key(&hk))
    }

    /// Sets self\[key\] to value.
    ///
    /// Note: raises OverflowError if the cache reached the maxsize limit,
    /// because this class does not have any algorithm.
    pub fn __setitem__(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();
        lock.insert(hk, value)?;
        Ok(())
    }

    /// Returns self\[key\]
    ///
    /// Note: raises KeyError if key not found.
    pub fn __getitem__(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.get(&hk) {
            Some(val) => Ok(val.clone_ref(py)),
            None => Err(err!(pyo3::exceptions::PyKeyError, hk.key)),
        }
    }

    /// Deletes self[key].
    ///
    /// Note: raises KeyError if key not found.
    pub fn __delitem__(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<()> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.remove(&hk) {
            Some(_) => Ok(()),
            None => Err(err!(pyo3::exceptions::PyKeyError, hk.key)),
        }
    }

    /// Returns repr(self)
    pub fn __repr__(&self) -> String {
        let lock = self.raw.lock();

        format!(
            "Cache({} / {}, capacity={})",
            lock.table.len(),
            lock.maxsize.get(),
            lock.table.capacity(),
        )
    }

    /// Returns iter(self)
    pub fn __iter__(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<cache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());
        let iter = unsafe { lock.table.iter() };

        let result = cache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(iter),
            typ: 0,
        };

        pyo3::Py::new(py, result)
    }

    /// Supports == and !=
    pub fn __richcmp__(
        slf: pyo3::PyRef<'_, Self>,
        other: pyo3::PyRef<'_, Self>,
        op: pyo3::class::basic::CompareOp,
    ) -> pyo3::PyResult<bool> {
        match op {
            pyo3::class::basic::CompareOp::Eq => {
                if slf.as_ptr() == other.as_ptr() {
                    return Ok(true);
                }

                let (a1, a2) = (slf.raw.lock(), other.raw.lock());
                Ok(a1.eq(&a2))
            }
            pyo3::class::basic::CompareOp::Ne => {
                if slf.as_ptr() == other.as_ptr() {
                    return Ok(false);
                }

                let (a1, a2) = (slf.raw.lock(), other.raw.lock());
                Ok(a1.ne(&a2))
            }
            _ => Err(err!(pyo3::exceptions::PyNotImplementedError, ())),
        }
    }

    pub fn __getstate__(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        let lock = self.raw.lock();
        unsafe {
            let state = lock.to_pickle(py)?;
            Ok(pyo3::Py::from_owned_ptr(py, state))
        }
    }

    pub fn __getnewargs__(&self) -> (usize,) {
        (0,)
    }

    pub fn __setstate__(&self, py: pyo3::Python<'_>, state: pyo3::PyObject) -> pyo3::PyResult<()> {
        let mut lock = self.raw.lock();
        unsafe { lock.from_pickle(py, state.as_ptr()) }
    }

    pub fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        for value in unsafe { self.raw.lock().table.iter() } {
            let (key, value) = unsafe { value.as_ref() };
            visit.call(&key.key)?;
            visit.call(value)?;
        }
        Ok(())
    }

    pub fn __clear__(&self) {
        let mut lock = self.raw.lock();
        lock.table.clear()
    }

    /// Returns the number of elements the map can hold without reallocating.
    pub fn capacity(&self) -> usize {
        let lock = self.raw.lock();
        lock.table.capacity()
    }

    /// Equivalent directly to `len(self) == self.maxsize`
    pub fn is_full(&self) -> bool {
        let lock = self.raw.lock();
        lock.table.len() == lock.maxsize.get()
    }

    /// Equivalent directly to `len(self) == 0`
    pub fn is_empty(&self) -> bool {
        let lock = self.raw.lock();
        lock.table.len() == 0
    }

    /// Equals to `self[key] = value`, but returns a value:
    ///
    /// - If the cache did not have this key present, None is returned.
    /// - If the cache did have this key present, the value is updated,
    ///   and the old value is returned. The key is not updated, though;
    ///
    /// Note: raises `OverflowError` if the cache reached the maxsize limit,
    /// because this class does not have any algorithm.
    pub fn insert(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();
        let op = lock.insert(hk, value)?;
        Ok(op.unwrap_or_else(|| py.None()))
    }

    /// Equals to `self[key]`, but returns `default` if the cache don't have this key present.
    #[pyo3(signature = (key, default=None))]
    pub fn get(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.get(&hk) {
            Some(val) => Ok(val.clone_ref(py)),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Removes specified key and return the corresponding value.
    ///
    /// If the key is not found, returns the default
    #[pyo3(signature = (key, default=None))]
    pub fn pop(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.remove(&hk) {
            Some((_, val)) => Ok(val),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Inserts key with a value of default if key is not in the cache.
    ///
    /// Return the value for key if key is in the cache, else default.
    #[pyo3(signature=(key, default=None))]
    pub fn setdefault(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        if let Some(x) = lock.get(&hk) {
            return Ok(x.clone_ref(py));
        }

        let defval = default.unwrap_or_else(|| py.None());
        lock.insert(hk, defval.clone_ref(py))?;
        Ok(defval)
    }

    /// not implemented
    pub fn popitem(&self) -> pyo3::PyResult<()> {
        Err(err!(pyo3::exceptions::PyNotImplementedError, ()))
    }

    /// not implemented
    #[allow(unused_variables)]
    pub fn drain(&self, n: usize) -> pyo3::PyResult<()> {
        Err(err!(pyo3::exceptions::PyNotImplementedError, ()))
    }

    /// Removes all items from cache.
    ///
    /// If reuse is True, will not free the memory for reusing in the future.
    #[pyo3(signature=(*, reuse=false))]
    pub fn clear(&self, reuse: bool) {
        let mut lock = self.raw.lock();
        lock.table.clear();

        if !reuse {
            lock.table.shrink_to(0, |x| x.0.hash);
        }
    }

    /// Shrinks the cache to fit len(self) elements.
    pub fn shrink_to_fit(&self) {
        let mut lock = self.raw.lock();
        lock.table.shrink_to(0, |x| x.0.hash)
    }

    /// Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
    ///
    /// Note: raises `OverflowError` if the cache reached the maxsize limit.
    pub fn update(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
        iterable: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        if slf.as_ptr() == iterable.as_ptr() {
            return Ok(());
        }

        let mut lock = slf.raw.lock();
        lock.update(py, iterable)
    }

    /// Returns an iterable object of the cache's items (key-value pairs).
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Items are not ordered.
    pub fn items(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<cache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());
        let iter = unsafe { lock.table.iter() };

        let result = cache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(iter),
            typ: 2,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns an iterable object of the cache's keys.
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Keys are not ordered.
    pub fn keys(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<cache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());
        let iter = unsafe { lock.table.iter() };

        let result = cache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(iter),
            typ: 0,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns an iterable object of the cache's values.
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Values are not ordered.
    pub fn values(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<cache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());
        let iter = unsafe { lock.table.iter() };

        let result = cache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(iter),
            typ: 1,
        };

        pyo3::Py::new(py, result)
    }
}

#[allow(non_camel_case_types)]
#[pyo3::pyclass(module = "cachebox._cachebox")]
pub struct cache_iterator {
    pub ptr: _KeepForIter<Cache>,
    pub iter: crate::mutex::Mutex<hashbrown::raw::RawIter<(HashedKey, pyo3::PyObject)>>,
    pub typ: u8,
}

#[pyo3::pymethods]
impl cache_iterator {
    pub fn __len__(&self) -> usize {
        self.ptr.len
    }

    pub fn __iter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> {
        slf
    }

    #[allow(unused_mut)]
    pub fn __next__(
        mut slf: pyo3::PyRefMut<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        slf.ptr.status(py)?;

        let mut l = slf.iter.lock();
        if let Some(x) = l.next() {
            let (key, val) = unsafe { x.as_ref() };

            match slf.typ {
                0 => return Ok(key.key.clone_ref(py).into_ptr()),
                1 => return Ok(val.clone_ref(py).into_ptr()),
                2 => {
                    return tuple!(
                        py,
                        2,
                        0 => key.key.clone_ref(py).into_ptr(),
                        1 => val.clone_ref(py).into_ptr(),
                    );
                }
                _ => {
                    #[cfg(not(debug_assertions))]
                    unsafe {
                        core::hint::unreachable_unchecked()
                    };
                    #[cfg(debug_assertions)]
                    unreachable!();
                }
            }
        }

        Err(err!(pyo3::exceptions::PyStopIteration, ()))
    }
}
