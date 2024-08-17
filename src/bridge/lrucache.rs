//! implement LRUCache, our lru implementation

use crate::{hashedkey::HashedKey, util::_KeepForIter};

/// LRU Cache implementation - Least recently used policy (thread-safe).
///
/// In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.
#[pyo3::pyclass(module="cachebox._cachebox", extends=crate::bridge::baseimpl::BaseCacheImpl)]
pub struct LRUCache {
    // Why [`Box`]? We using [`Box`] here so that there's no need for `&mut self`
    // in this struct; so RuntimeError never occurred for using this class in multiple threads.
    raw: Box<crate::mutex::Mutex<crate::internal::LRUPolicy>>,
}

#[pyo3::pymethods]
impl LRUCache {
    /// LRU Cache implementation - Least recently used policy (thread-safe).
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
        let mut raw = crate::internal::LRUPolicy::new(maxsize, capacity)?;
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

        core::mem::size_of::<Self>()
            + lock.table.capacity()
                * core::mem::size_of::<core::ptr::NonNull<crate::linked_list::Node>>()
            + lock.list.len() * core::mem::size_of::<crate::linked_list::Node>()
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
    pub fn __setitem__(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();
        lock.insert(hk, value);
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
        let mut lock = self.raw.lock();

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

    /// Returns str(self)
    pub fn __str__(&self) -> String {
        let lock = self.raw.lock();

        format!(
            "LRUCache({} / {}, capacity={})",
            lock.table.len(),
            lock.maxsize.get(),
            lock.table.capacity(),
        )
    }

    /// Returns iter(self)
    pub fn __iter__(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lrucache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lrucache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.list.iter()),
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
        unsafe {
            for bucket in self.raw.lock().table.iter() {
                let node = bucket.as_ref();

                visit.call(&(*node.as_ptr()).element.0.key)?;
                visit.call(&(*node.as_ptr()).element.1)?;
            }
        }

        Ok(())
    }

    pub fn __clear__(&self) {
        let mut lock = self.raw.lock();
        lock.table.clear();
        lock.list.clear();
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
    pub fn insert(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();
        let op = lock.insert(hk, value);
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
        let mut lock = self.raw.lock();

        match lock.get(&hk) {
            Some(val) => Ok(val.clone_ref(py)),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    /// Searches for a key-value in the cache and returns it (without moving the key to recently used).
    #[pyo3(signature = (key, default=None))]
    pub fn peek(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: Option<pyo3::PyObject>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let hk = HashedKey::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.peek(&hk) {
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
        lock.insert(hk, defval.clone_ref(py));
        Ok(defval)
    }

    /// Removes the element that has been in the cache the longest
    pub fn popitem(&self) -> pyo3::PyResult<(pyo3::PyObject, pyo3::PyObject)> {
        let mut lock = self.raw.lock();
        match lock.popitem() {
            Some((key, val)) => Ok((key.key, val)),
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
        lock.list.clear();

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
    pub fn items(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lrucache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lrucache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.list.iter()),
            typ: 2,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns an iterable object of the cache's keys.
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    pub fn keys(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lrucache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lrucache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.list.iter()),
            typ: 0,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns an iterable object of the cache's values.
    ///
    /// Notes:
    /// - You should not make any changes in cache while using this iterable object.
    pub fn values(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lrucache_iterator>> {
        let lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lrucache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.list.iter()),
            typ: 1,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns the key in the cache that has not been accessed in the longest time.
    #[pyo3(signature=(n=0))]
    pub fn least_recently_used(
        &self,
        py: pyo3::Python<'_>,
        mut n: usize,
    ) -> Option<pyo3::PyObject> {
        let lock = self.raw.lock();

        if n >= lock.list.len() {
            None
        } else {
            let mut node = lock.list.head?;

            unsafe {
                while n > 0 {
                    debug_assert!((*node.as_ptr()).next.is_some()); // we checked length, so it have to available
                    node = (*node.as_ptr()).next.unwrap();
                    n -= 1;
                }

                Some((*node.as_ptr()).element.0.key.clone_ref(py))
            }
        }
    }

    /// Returns the key in the cache that has been accessed in the shortest time.
    pub fn most_recently_used(&self, py: pyo3::Python<'_>) -> Option<pyo3::PyObject> {
        let lock = self.raw.lock();
        lock.list
            .tail
            .map(|node| unsafe { (*node.as_ptr()).element.0.key.clone_ref(py) })
    }
}

#[allow(non_camel_case_types)]
#[pyo3::pyclass(module = "cachebox._cachebox")]
pub struct lrucache_iterator {
    ptr: _KeepForIter<LRUCache>,
    iter: crate::mutex::Mutex<crate::linked_list::Iter>,
    typ: u8,
}

#[pyo3::pymethods]
impl lrucache_iterator {
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
                let (key, val) = unsafe { &(*ptr.as_ptr()).element };

                match slf.typ {
                    0 => Ok(key.key.clone_ref(py).into_ptr()),
                    1 => Ok(val.clone_ref(py).into_ptr()),
                    2 => {
                        tuple!(
                            py,
                            2,
                            0 => key.key.clone_ref(py).into_ptr(),
                            1 => val.clone_ref(py).into_ptr(),
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
