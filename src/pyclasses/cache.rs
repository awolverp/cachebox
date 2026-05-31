use crate::internal::alias;
use crate::internal::onceinit;
use crate::internal::utils;
use crate::policies::nopolicy;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;
use crate::policies::wrapped::Wrapped;

implement_pyclass! {
    /// A thread-safe, memory-efficient key-value cache with no eviction policy.
    ///
    /// Items remain in the cache until manually removed or the cache is cleared.
    ///
    /// ``Cache`` is essentially a configurable hashmap-like store. When an item is
    /// inserted, it is stored directly without any ordering, priority tracking, or
    /// access metadata. If a maximum size is configured, insertions beyond that
    /// limit are rejected with an ``OverflowError``. All read and write operations
    /// are thread-safe.
    ///
    /// Because no eviction logic runs in the background, there is no overhead from
    /// tracking usage order, frequency counters, or expiry timestamps.
    ///
    /// Pros:
    ///     - Minimal overhead: no bookkeeping for eviction means lower CPU and
    ///       memory usage per entry compared to policy-based caches.
    ///     - Predictable behavior: items are never silently removed, so cache hits
    ///       are deterministic once an item is stored.
    ///     - Thread-safe: safe for concurrent reads and writes out of the box.
    ///     - Configurable capacity: a hard size limit prevents unbounded memory
    ///       growth.
    ///
    /// Cons:
    ///     - No automatic eviction: the cache can fill up and stop accepting new
    ///       entries if a max size is set, requiring manual management.
    ///     - Unordered: unlike a standard ``dict`` (Python 3.7+), insertion order
    ///       is not preserved.
    ///     - Not suitable for volatile data: stale entries persist forever unless
    ///       explicitly invalidated.
    ///
    /// Use ``Cache`` when you have a fixed, well-known set of keys that are
    /// expensive to compute and never go stale (e.g. parsed config values,
    /// compiled regex patterns, loaded templates), and when the lowest possible
    /// overhead is required.
    ///
    /// Avoid it when cached data can become stale, when the working set is
    /// unpredictable in size, or when automatic memory pressure relief is needed.
    [subclass, extends=crate::pyclasses::base::PyBaseCacheImpl, generic, frozen]
    PyCache as "Cache" (onceinit::OnceInit<Wrapped<nopolicy::NoPolicy>>);
}

#[pyo3::pymethods]
impl PyCache {
    #[new]
    #[allow(unused_variables)]
    #[pyo3(signature=(*args, **kwds))]
    fn __new__(
        args: alias::ArgsType,
        kwds: Option<alias::KwdsType>,
    ) -> (Self, crate::pyclasses::base::PyBaseCacheImpl) {
        (
            Self(onceinit::OnceInit::uninit()),
            crate::pyclasses::base::PyBaseCacheImpl,
        )
    }

    /// Initialize a new `Cache` instance.
    ///
    /// Args:
    ///     maxsize: Maximum number of elements the cache can hold.
    ///     iterable: Initial data to populate the cache.
    ///     capacity: Pre-allocate hash table capacity to minimize reallocations. Defaults to 0.
    ///     getsizeof: A callable that computes the size of a key-value pair. When `None`, each
    ///             entry is assumed to have a size of 1 (equivalent to `lambda k, v: 1`).
    ///             Use this to implement weighted caching — for example, sizing entries by
    ///             memory footprint or byte length.
    ///
    /// The cache can be pre-sized via `capacity` to reduce hash table reallocations when
    /// the number of expected entries is known ahead of time.
    #[pyo3(signature=(maxsize, iterable=None, *, capacity=0, getsizeof=None))]
    fn __init__(
        &self,
        py: pyo3::Python,
        maxsize: usize,
        iterable: Option<alias::BoundObject>,
        capacity: usize,
        getsizeof: Option<alias::PyObject>,
    ) -> pyo3::PyResult<()> {
        let wrapped = Wrapped::new(
            nopolicy::NoPolicy::new(capacity),
            nopolicy::Shared::new(maxsize, getsizeof),
        );

        // Populate cache if `iterable` passed
        let extend_result = {
            if let Some(iterable) = iterable {
                let getsizeof = wrapped.shared().getsizeof().clone_ref(py);

                let result = wrapped.extend(
                    // iterable object
                    iterable,
                    // transform function
                    |key, value| nopolicy::Handle::new(py, &getsizeof, key, value),
                );
                result
            } else {
                Ok(())
            }
        };

        self.0.set(wrapped);
        extend_result
    }

    #[getter]
    #[inline]
    fn maxsize(&self) -> usize {
        let inner = self.0.get();
        inner.shared().maxsize()
    }

    #[inline]
    fn current_size(&self) -> usize {
        let inner = self.0.get();
        inner.policy().current_size()
    }

    #[inline]
    fn remaining_size(&self) -> usize {
        let inner = self.0.get();
        inner.remaining_size()
    }

    #[getter]
    #[inline]
    fn getsizeof(&self, py: pyo3::Python) -> Option<alias::PyObject> {
        let inner = self.0.get();
        inner.shared().getsizeof().clone_ref(py).into()
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    fn capacity(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        policy.table().capacity()
    }

    /// Returns the number of entries currently in the cache.
    #[inline]
    fn __len__(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        policy.table().len()
    }

    #[inline]
    fn __sizeof__(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        policy.table().capacity() * std::mem::size_of::<nopolicy::Handle>()
    }

    #[inline]
    fn __bool__(&self) -> bool {
        let inner = self.0.get();
        let policy = inner.policy();

        !policy.table().is_empty()
    }

    #[inline]
    fn __contains__(&self, py: pyo3::Python, key: alias::PyObject) -> pyo3::PyResult<bool> {
        self.contains(py, key)
    }

    /// Returns `true` if the cache contains an entry for `key`.
    #[inline]
    fn contains(&self, py: pyo3::Python, key: alias::PyObject) -> pyo3::PyResult<bool> {
        let key = utils::PrecomputedHashObject::new(py, key)?;
        let inner = self.0.get();
        inner.contains(py, &key)
    }

    /// Returns `True` if cache is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        let inner = self.0.get();
        let policy = inner.policy();

        policy.table().is_empty()
    }

    /// Returns `True` when the cumulative size has reached the maxsize limit.
    #[inline]
    fn is_full(&self) -> bool {
        let inner = self.0.get();
        let shared = inner.shared();
        let policy = inner.policy();

        policy.current_size() >= shared.maxsize()
    }

    /// Equals to `self[key] = value`, but returns a value:
    ///
    /// - If the cache did not have this key present, None is returned.
    /// - If the cache did have this key present, the value is updated,
    ///   and the old value is returned. The key is not updated, though.
    ///
    /// Note: raises `OverflowError` if the cache reached the maxsize limit,
    /// because this class does not have any algorithm.
    fn insert(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Option<alias::PyObject>> {
        let inner = self.0.get();
        let handle = nopolicy::Handle::new(py, inner.shared().getsizeof(), key, value)?;

        let old_handle = inner.insert(py, handle)?.map(|x| x.into_value());
        Ok(old_handle)
    }

    /// Updates the cache with elements from a dictionary or an iterable object of key/value pairs.
    fn update(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python,
        iterable: alias::PyObject,
    ) -> pyo3::PyResult<()> {
        if std::ptr::eq(slf.as_ptr(), iterable.as_ptr()) {
            return Ok(());
        }

        let inner = slf.0.get();
        let getsizeof = inner.shared().getsizeof().clone_ref(py);

        inner.extend(
            // iterable object
            iterable.into_bound(py),
            // transform function
            move |key, value| nopolicy::Handle::new(py, &getsizeof, key, value),
        )
    }

    #[inline]
    fn __setitem__(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<()> {
        self.insert(py, key, value)?;
        Ok(())
    }

    /// Retrieves the value for a given key from the cache.
    ///
    /// Returns the value associated with the key if present, otherwise returns the specified default value.
    /// Equivalent to `self[key]`, but provides a fallback default if the key is not found.
    ///
    /// Args:
    ///     key: The key to look up in the cache.
    ///     default: The value to return if the key is not present in the cache. Defaults to None.
    ///
    /// Returns:
    ///     The value associated with the key, or the default value if the key is not found.
    #[pyo3(signature = (key, default=utils::OptionalArgument::Undefined))]
    fn get(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        default: utils::OptionalArgument,
    ) -> pyo3::PyResult<alias::PyObject> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();
        let mut policy = inner.policy();

        if let Some(x) = policy.get(py, &key)? {
            return Ok(x.value().clone_ref(py));
        }

        match default {
            utils::OptionalArgument::Defined(x) => Ok(x),
            utils::OptionalArgument::Undefined => unsafe {
                // SAFETY: None is immortal, so reference counting has no meaning
                Ok(pyo3::Bound::from_owned_ptr(py, pyo3::ffi::Py_None()).unbind())
            },
        }
    }

    fn __getitem__(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
    ) -> pyo3::PyResult<alias::PyObject> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();
        let mut policy = inner.policy();

        match policy.get(py, &key)? {
            Some(x) => Ok(x.value().clone_ref(py)),
            None => Err(new_py_error!(
                PyKeyError,
                Into::<alias::PyObject>::into(key)
            )),
        }
    }

    /// Inserts key with a value of default if key is not in the cache.
    ///
    /// Returns the value for key if key is in the cache, else default.
    #[pyo3(signature = (key, default=utils::OptionalArgument::Undefined))]
    fn setdefault(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        default: utils::OptionalArgument,
    ) -> pyo3::PyResult<alias::PyObject> {
        // 1. Try to get value
        // 2. If exists -> return it
        // 3. Else -> insert default -> return default
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();
        let shared = inner.shared();
        let mut policy = inner.policy();

        if let Some(x) = policy.get(py, &key)? {
            return Ok(x.value().clone_ref(py));
        }
        drop(policy);

        let default_object = match default {
            utils::OptionalArgument::Defined(x) => x,
            utils::OptionalArgument::Undefined => unsafe {
                // SAFETY: None is immortal, so reference counting has no meaning
                pyo3::Bound::from_owned_ptr(py, pyo3::ffi::Py_None()).unbind()
            },
        };

        let handle = nopolicy::Handle::with_precomputed_hash_key(
            py,
            shared.getsizeof(),
            key,
            default_object.clone_ref(py),
        )?;

        inner.insert(py, handle)?;
        Ok(default_object)
    }

    /// Removes specified key and returns the corresponding value.
    ///
    /// If the key is not found, returns the `default` if given; otherwise, raise a KeyError.
    #[pyo3(signature = (key, default=utils::OptionalArgument::Undefined))]
    fn pop(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        default: utils::OptionalArgument,
    ) -> pyo3::PyResult<alias::PyObject> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();

        if let Some(x) = inner.remove(py, &key)? {
            return Ok(x.into_value());
        }

        match default {
            utils::OptionalArgument::Defined(x) => Ok(x),
            utils::OptionalArgument::Undefined => Err(new_py_error!(
                PyKeyError,
                Into::<alias::PyObject>::into(key)
            )),
        }
    }

    fn __delitem__(&self, py: pyo3::Python, key: alias::PyObject) -> pyo3::PyResult<()> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();
        match inner.remove(py, &key)? {
            Some(_) => Ok(()),
            None => Err(new_py_error!(
                PyKeyError,
                Into::<alias::PyObject>::into(key)
            )),
        }
    }

    /// Remove and return a (key, value) pair as a 2-tuple.
    ///
    /// NOTE: `Cache` always raises `NotImplementedError` because has neither policy nor algorithm to evict items.
    fn popitem(&self) -> pyo3::PyResult<(alias::PyObject, alias::PyObject)> {
        let inner = self.0.get();
        let mut policy = inner.policy();

        let handle = policy.evict(inner.shared())?;
        drop(policy);

        let (key, val) = handle.into_pair();
        Ok((key.into(), val))
    }

    /// Calls the `popitem()` `n` times and returns count of removed items.
    #[inline]
    fn drain(
        &self,
        py: pyo3::Python,
        n: pyo3::ffi::Py_ssize_t,
    ) -> pyo3::PyResult<pyo3::ffi::Py_ssize_t> {
        let inner = self.0.get();
        inner.drain(py, n)
    }

    /// Shrinks the internal allocation as close to the current length as possible.
    #[inline]
    fn shrink_to_fit(&self) {
        let inner = self.0.get();
        let mut policy = inner.policy();
        policy.shrink_to_fit(inner.shared());
    }

    /// Removes all entries from the table and resets the cumulative size to zero.
    #[pyo3(signature=(*, reuse=false))]
    fn clear(&self, reuse: bool) {
        let inner = self.0.get();
        let shared = inner.shared();
        let mut policy = inner.policy();

        policy.clear(shared);

        if !reuse {
            policy.shrink_to_fit(shared);
        }
    }

    fn __eq__(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python,
        other: pyo3::PyRef<'_, Self>,
    ) -> pyo3::PyResult<bool> {
        if std::ptr::eq(slf.as_ptr(), other.as_ptr()) {
            return Ok(true);
        }

        let self_inner = slf.0.get();
        let other_inner = other.0.get();

        let self_policy = self_inner.policy();
        let other_policy = other_inner.policy();

        self_policy.py_eq(
            py,
            self_inner.shared(),
            &*other_policy,
            other_inner.shared(),
        )
    }

    fn __ne__(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python,
        other: pyo3::PyRef<'_, Self>,
    ) -> pyo3::PyResult<bool> {
        if std::ptr::eq(slf.as_ptr(), other.as_ptr()) {
            return Ok(false);
        }

        let self_inner = slf.0.get();
        let other_inner = other.0.get();

        let self_policy = self_inner.policy();
        let other_policy = other_inner.policy();

        self_policy
            .py_eq(
                py,
                self_inner.shared(),
                &*other_policy,
                other_inner.shared(),
            )
            .map(|x| !x)
    }

    fn items(&self) -> pyo3::PyResult<pyo3::Py<PyCacheItems>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyCacheItems {
            iter: parking_lot::Mutex::new(unsafe { inner.policy().table().iter() }),
            gv,
            initial_gv,
        };

        pyo3::Python::attach(|py| pyo3::Py::new(py, result))
    }

    fn values(&self) -> pyo3::PyResult<pyo3::Py<PyCacheValues>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyCacheValues {
            iter: parking_lot::Mutex::new(unsafe { inner.policy().table().iter() }),
            gv,
            initial_gv,
        };
        pyo3::Python::attach(|py| pyo3::Py::new(py, result))
    }

    fn keys(&self) -> pyo3::PyResult<pyo3::Py<PyCacheKeys>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyCacheKeys {
            iter: parking_lot::Mutex::new(unsafe { inner.policy().table().iter() }),
            gv,
            initial_gv,
        };
        pyo3::Python::attach(|py| pyo3::Py::new(py, result))
    }

    #[inline]
    fn __iter__(&self) -> pyo3::PyResult<pyo3::Py<PyCacheKeys>> {
        self.keys()
    }

    fn copy(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<Self>> {
        let inner = self.0.get();
        let cloned = inner.clone_ref(py);
        let result = Self(onceinit::OnceInit::new(cloned));

        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseCacheImpl))
    }

    #[inline]
    fn __copy__(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<Self>> {
        self.copy(py)
    }

    fn __getstate__(&self, py: pyo3::Python) -> pyo3::PyResult<alias::PyObject> {
        let inner = self.0.get();
        inner.build_pickle(py).map(|x| x.into())
    }

    fn __setstate__(&self, py: pyo3::Python, state: alias::PyObject) -> pyo3::PyResult<()> {
        let wrapped = Wrapped::from_pickle(py, state)?;
        self.0.set(wrapped);
        Ok(())
    }

    fn __repr__(slf: pyo3::PyRef<'_, Self>, py: pyo3::Python) -> String {
        let inner = slf.0.get();
        let shared = inner.shared();
        let policy = inner.policy();

        let iter = unsafe {
            policy
                .table()
                .iter()
                .map(|bucket| bucket.as_ref())
                .map(|handle| {
                    (
                        // Without using `.bind` it returns something like `Py(addr)`
                        handle.key().as_ref().bind(py),
                        handle.value().bind(py),
                    )
                })
        };

        let items = utils::items_to_str(iter, policy.table().len()).unwrap();
        format!(
            "{}[maxsize={}]({})",
            unsafe { utils::get_type_name(py, slf.as_ptr()) },
            shared.maxsize(),
            items
        )
    }

    fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        if self.0.is_initialized() {
            return Ok(());
        }

        let inner = self.0.get();
        let policy = inner.policy();

        for handle_ref in unsafe { policy.table().iter() } {
            let handle = unsafe { handle_ref.as_ref() };

            visit.call(handle.key().as_ref())?;
            visit.call(handle.value())?;
        }
        Ok(())
    }

    fn __clear__(&self) {
        if self.0.is_initialized() {
            return;
        }

        let inner = self.0.get();
        let mut policy = inner.policy();
        policy.clear(inner.shared());
    }
}

// Implement iterators
macro_rules! implement_iterator {
    (
        $(
            $name:ident as $pyname:literal
            fn ($py:ident, $handle:ident) -> $rt_type:ty { $init:expr }
        )+
    ) => {
        $(
            implement_pyclass! {
                [generic, frozen] $name as $pyname {
                    initial_gv: u32,
                    gv: utils::GenerationVersion,
                    iter: parking_lot::Mutex<crate::hashbrown::raw::RawIter<nopolicy::Handle>>,
                }
            }

            #[pyo3::pymethods]
            impl $name {
                #[inline]
                fn __iter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> {
                    slf
                }

                fn __next__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyResult<$rt_type> {
                    if slf.initial_gv != slf.gv.get() {
                        return Err(new_py_error!(
                            PyRuntimeError,
                            "cache size changed during iteration"
                        ));
                    }

                    let mut iter = slf.iter.lock();

                    match iter.next() {
                        Some(x) => {
                            let $py = slf.py();
                            let $handle = unsafe { x.as_ref() };
                            Ok($init)
                        }
                        None => return Err(new_py_error!(PyStopIteration, ())),
                    }
                }
            }
        )+
    };
}
implement_iterator!(
    PyCacheItems as "cache_items"
    fn(py, handle) -> (alias::PyObject, alias::PyObject) {{
        let (key, val) = handle.clone_ref(py).into_pair();
        (key.into(), val)
    }}

    PyCacheKeys as "cache_keys"
    fn(py, handle) -> alias::PyObject { handle.key().clone_ref(py).into() }

    PyCacheValues as "cache_values"
    fn(py, handle) -> alias::PyObject { handle.value().clone_ref(py) }
);
