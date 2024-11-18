//! implement LFUCache, our lfu implementation

use crate::{hashedkey::HashedKey, util::_KeepForIter};

/// LFU Cache implementation - Least frequantly used policy (thread-safe).
///
/// In simple terms, the LFU cache will remove the element in the cache that has been accessed the least, regardless of time
#[pyo3::pyclass(module="cachebox._cachebox", extends=crate::bridge::baseimpl::BaseCacheImpl, frozen)]
pub struct LFUCache {
    // Why [`Box`]? We using [`Box`] here so that there's no need for `&mut self`
    // in this struct; so RuntimeError never occurred for using this class in multiple threads.
    raw: Box<crate::mutex::Mutex<crate::internal::LFUPolicy>>,
}

#[pyo3::pymethods]
impl LFUCache {
    /// LFU Cache implementation - Least frequantly used policy (thread-safe).
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
        let mut raw = crate::internal::LFUPolicy::new(maxsize, capacity)?;
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
                * core::mem::size_of::<
                    core::ptr::NonNull<
                        crate::sorted_heap::Entry<(HashedKey, pyo3::PyObject, usize)>,
                    >,
                >()
            + lock.heap.capacity()
                * core::mem::size_of::<crate::sorted_heap::Entry<(HashedKey, pyo3::PyObject, usize)>>(
                )
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

    /// Returns repr(self)
    pub fn __repr__(&self) -> String {
        let lock = self.raw.lock();

        format!(
            "LFUCache({} / {}, capacity={})",
            lock.table.len(),
            lock.maxsize.get(),
            lock.table.capacity(),
        )
    }

    /// Returns iter(self)
    pub fn __iter__(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lfucache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lfucache_iterator {
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

                visit.call(&(*node.as_ptr()).as_ref().0.key)?;
                visit.call(&(*node.as_ptr()).as_ref().1)?;
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
            Some((_, val, _)) => Ok(val),
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
            Some((key, val, _)) => Ok((key.key, val)),
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
    ) -> pyo3::PyResult<pyo3::Py<lfucache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lfucache_iterator {
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
    pub fn keys(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lfucache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lfucache_iterator {
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
    pub fn values(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<lfucache_iterator>> {
        let mut lock = slf.raw.lock();
        let (len, capacity) = (lock.table.len(), lock.table.capacity());

        let result = lfucache_iterator {
            ptr: _KeepForIter::new(slf.as_ptr(), capacity, len),
            iter: crate::mutex::Mutex::new(lock.iter()),
            typ: 1,
        };

        pyo3::Py::new(py, result)
    }

    /// Returns the key in the cache that has been accessed the least, regardless of time.
    #[pyo3(signature=(n=0))]
    pub fn least_frequently_used(&self, py: pyo3::Python<'_>, n: usize) -> Option<pyo3::PyObject> {
        let mut lock = self.raw.lock();
        lock.heap.sort(|a, b| a.2.cmp(&b.2));
        let node = lock.heap.get(n)?;

        Some(unsafe { (*node.as_ptr()).as_ref().0.key.clone_ref(py) })
    }
}

#[allow(non_camel_case_types)]
#[pyo3::pyclass(module = "cachebox._cachebox")]
pub struct lfucache_iterator {
    ptr: _KeepForIter<LFUCache>,
    iter: crate::mutex::Mutex<crate::sorted_heap::Iter<(HashedKey, pyo3::PyObject, usize)>>,
    typ: u8,
}

#[pyo3::pymethods]
impl lfucache_iterator {
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
                    0 => Ok(node.as_ref().0.key.clone_ref(py).into_ptr()),
                    1 => Ok(node.as_ref().1.clone_ref(py).into_ptr()),
                    2 => {
                        tuple!(
                            py,
                            2,
                            0 => node.as_ref().0.key.clone_ref(py).into_ptr(),
                            1 => node.as_ref().1.clone_ref(py).into_ptr(),
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
