//! implement VTTLCache, our vttl implementation

use crate::{hashedkey::HashedKey, internal::VTTLElement, util::_KeepForIter};

/// VTTL Cache Implementation - Time-To-Live Per-Key Policy (thread-safe).
///
/// In simple terms, the TTL cache will automatically remove the element in the cache that has expired when need.
#[pyo3::pyclass(module="cachebox._cachebox", extends=crate::bridge::baseimpl::BaseCacheImpl)]
pub struct VTTLCache {
    // Why [`Box`]? We using [`Box`] here so that there's no need for `&mut self`
    // in this struct; so RuntimeError never occurred for using this class in multiple threads.
    raw: Box<crate::mutex::Mutex<crate::internal::VTTLPolicy>>,
}

#[pyo3::pymethods]
impl VTTLCache {
    /// VTTL Cache Implementation - Time-To-Live Per-Key Policy (thread-safe).
    ///
    /// By maxsize param, you can specify the limit size of the cache ( zero means infinity ); this is unchangable.
    ///
    /// By iterable param, you can create cache from a dict or an iterable.
    ///
    /// If capacity param is given, cache attempts to allocate a new hash table with at
    /// least enough capacity for inserting the given number of elements without reallocating.
    #[new]
    #[pyo3(signature=(maxsize, iterable=None, ttl=None, *, capacity=0))]
    pub fn __new__(
        py: pyo3::Python<'_>,
        maxsize: usize,
        iterable: Option<pyo3::PyObject>,
        ttl: Option<f64>,
        capacity: usize,
    ) -> pyo3::PyResult<(Self, crate::bridge::baseimpl::BaseCacheImpl)> {
        let mut raw = crate::internal::VTTLPolicy::new(maxsize, capacity)?;
        if iterable.is_some() {
            raw.update(py, unsafe { iterable.unwrap_unchecked() }, ttl)?;
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
        let mut lock = self.raw.lock();
        lock.expire();
        lock.table.len()
    }

    /// Returns allocated memory size - sys.getsizeof(self)
    pub fn __sizeof__(&self) -> usize {
        let lock = self.raw.lock();

        core::mem::size_of::<Self>()
            + lock.table.capacity()
                * core::mem::size_of::<core::ptr::NonNull<crate::sorted_heap::Entry<VTTLElement>>>()
            + lock.heap.capacity() * core::mem::size_of::<crate::sorted_heap::Entry<VTTLElement>>()
    }

    /// Returns true if cache not empty - bool(self)
    pub fn __bool__(&self) -> bool {
        let mut lock = self.raw.lock();
        lock.expire();
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
    /// Recommended to use `.insert()` method here.
    pub fn __setitem__(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();
        lock.insert(hk, value, None, true);
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
            Some(val) => Ok(val.value.clone_ref(py)),
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

    /// Returns str(self)
    pub fn __str__(&self) -> String {
        let lock = self.raw.lock();

        format!(
            "VTTLCache({} / {}, capacity={})",
            lock.table.len(),
            lock.maxsize.get(),
            lock.table.capacity(),
        )
    }

    /// Returns `iter(cache)`
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
    pub fn __iter__(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<vttlcache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = vttlcache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.iter()),
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
        let mut lock = self.raw.lock();
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
        unsafe {
            for bucket in self.raw.lock().table.iter() {
                let node = bucket.as_ref();

                visit.call(&(*node.as_ptr()).as_ref().key.key)?;
                visit.call(&(*node.as_ptr()).as_ref().value)?;
            }
        }

        Ok(())
    }

    pub fn __clear__(&self) {
        let mut lock = self.raw.lock();
        lock.table.clear();
        lock.heap.clear();
    }

    /// Returns the number of elements the map can hold without reallocating.
    pub fn capacity(&self) -> usize {
        let lock = self.raw.lock();
        lock.table.capacity()
    }

    /// Equivalent directly to `len(self) == self.maxsize`
    pub fn is_full(&self) -> bool {
        let mut lock = self.raw.lock();
        lock.expire();
        lock.table.len() == lock.maxsize.get()
    }

    /// Equivalent directly to `len(self) == 0`
    pub fn is_empty(&self) -> bool {
        let mut lock = self.raw.lock();
        lock.expire();
        lock.table.len() == 0
    }

    /// Equals to `self[key] = value`, but:
    /// - Here you can set ttl for key-value ( with `self[key] = value` you can't )
    /// - If the cache did not have this key present, None is returned.
    /// - If the cache did have this key present, the value is updated,
    ///   and the old value is returned. The key is not updated, though;
    #[pyo3(signature=(key, value, ttl=None))]
    pub fn insert(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        if let Some(secs) = ttl {
            if secs == 0.0 {
                return Err(err!(
                    pyo3::exceptions::PyValueError,
                    "ttl cannot be zero, if you do not want to set ttl, use `None`"
                ));
            } else if secs < 0.0 {
                return Err(err!(
                    pyo3::exceptions::PyValueError,
                    "ttl cannot be negative"
                ));
            }
        }

        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();
        let op = lock.insert(hk, value, ttl, true);
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
            Some(val) => Ok(val.value.clone_ref(py)),
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
            Some(element) => Ok(element.value),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Inserts key with a value of default if key is not in the cache.
    ///
    /// Return the value for key if key is in the cache, else default.
    #[pyo3(signature=(key, default=None, ttl=None))]
    pub fn setdefault(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        if let Some(x) = lock.get(&hk) {
            return Ok(x.value.clone_ref(py));
        }

        let defval = default.unwrap_or_else(|| py.None());
        lock.insert(hk, defval.clone_ref(py), ttl, true);
        Ok(defval)
    }

    /// Removes the element that has been in the cache the longest
    pub fn popitem(&self) -> pyo3::PyResult<(pyo3::PyObject, pyo3::PyObject)> {
        let mut lock = self.raw.lock();

        match lock.popitem() {
            Some(element) => Ok((element.key.key, element.value)),
            None => Err(err!(pyo3::exceptions::PyKeyError, ())),
        }
    }

    /// Does the `popitem()` `n` times and returns count of removed items.
    pub fn drain(&self, n: usize) -> usize {
        let mut lock = self.raw.lock();

        for c in 0..n {
            if lock.popitem().is_none() {
                return c;
            }
        }

        0
    }

    /// Removes all items from cache.
    ///
    /// If reuse is True, will not free the memory for reusing in the future.
    #[pyo3(signature=(*, reuse=false))]
    pub fn clear(&self, reuse: bool) {
        let mut lock = self.raw.lock();
        lock.table.clear();
        lock.heap.clear();

        if !reuse {
            lock.shrink_to_fit();
        }
    }

    /// Shrinks the cache to fit len(self) elements.
    pub fn shrink_to_fit(&self) {
        let mut lock = self.raw.lock();
        lock.shrink_to_fit();
    }

    /// Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
    #[pyo3(signature=(iterable, ttl=None))]
    pub fn update(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
        iterable: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<()> {
        if slf.as_ptr() == iterable.as_ptr() {
            return Ok(());
        }

        let mut lock = slf.raw.lock();
        lock.update(py, iterable, ttl)
    }

    /// Returns an iterable object of the cache's items (key-value pairs).
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
    pub fn items(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<vttlcache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = vttlcache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.iter()),
            typ: 2,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns an iterable object of the cache's keys.
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
    pub fn keys(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<vttlcache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = vttlcache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.iter()),
            typ: 0,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns an iterable object of the cache's values.
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    /// - Don't call `len(cache)`, `bool(cache)`, `cache.is_full()` or `cache.is_empty()` while using this iterable object.
    pub fn values(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<vttlcache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = vttlcache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.iter()),
            typ: 1,
        };

        pyo3::Py::new(py, result)
    }

    /// Works like `.get()`, but also returns the remaining time-to-live.
    #[pyo3(signature = (key, default=None))]
    pub fn get_with_expire(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
    ) -> pyo3::PyResult<(pyo3::PyObject, f64)> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.get(&hk) {
            Some(val) => Ok((val.value.clone_ref(py), unsafe {
                val.expire_at.map_or(0.0, |x| {
                    x.duration_since(std::time::SystemTime::now())
                        .unwrap_unchecked()
                        .as_secs_f64()
                })
            })),
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    /// Works like `.pop()`, but also returns the remaining time-to-live.
    #[pyo3(signature = (key, default=None))]
    pub fn pop_with_expire(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
    ) -> pyo3::PyResult<(pyo3::PyObject, f64)> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.remove(&hk) {
            #[rustfmt::skip]
            Some(element) => Ok(
                (
                    element.value,
                    unsafe {
                        element.expire_at.map_or(
                            0.0, |x| {
                                x.duration_since(std::time::SystemTime::now())
                                .unwrap_unchecked()
                                .as_secs_f64()
                            }
                        )
                    }
                )
            ),
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    /// Works like `.popitem()`, but also returns the remaining time-to-live.
    pub fn popitem_with_expire(&self) -> pyo3::PyResult<(pyo3::PyObject, pyo3::PyObject, f64)> {
        let mut lock = self.raw.lock();
        match lock.popitem() {
            #[rustfmt::skip]
            Some(element) => Ok(
                (
                    element.key.key,
                    element.value,
                    unsafe {
                        element.expire_at.map_or(
                            0.0, |x| {
                                x.duration_since(std::time::SystemTime::now())
                                .unwrap_unchecked()
                                .as_secs_f64()
                            }
                        )
                    }
                )
            ),
            None => Err(err!(pyo3::exceptions::PyKeyError, ())),
        }
    }
}

#[allow(non_camel_case_types)]
#[pyo3::pyclass(module = "cachebox._cachebox")]
pub struct vttlcache_iterator {
    ptr: _KeepForIter<VTTLCache>,
    iter: crate::mutex::Mutex<crate::sorted_heap::Iter<VTTLElement>>,
    typ: u8,
}

#[pyo3::pymethods]
impl vttlcache_iterator {
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

        match slf.iter.lock().next() {
            Some(ptr) => {
                let node = unsafe { &*ptr.as_ptr() };

                match slf.typ {
                    0 => Ok(node.as_ref().key.key.clone_ref(py).into_ptr()),
                    1 => Ok(node.as_ref().value.clone_ref(py).into_ptr()),
                    2 => {
                        tuple!(
                            py,
                            2,
                            0 => node.as_ref().key.key.clone_ref(py).into_ptr(),
                            1 => node.as_ref().value.clone_ref(py).into_ptr(),
                        )
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
            None => Err(err!(pyo3::exceptions::PyStopIteration, ())),
        }
    }
}
