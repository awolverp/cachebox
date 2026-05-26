use crate::internal::alias;
use crate::internal::onceinit;
use crate::internal::utils;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;
use crate::policies::ttlpolicy;
use crate::policies::wrapped::Wrapped;

implement_pyclass! {
    /// A Time-To-Live (TTL) cache eviction policy: each entry carries an expiration timestamp
    /// and is considered stale — and eligible for eviction — once that deadline has passed,
    /// regardless of how recently or frequently it was accessed.
    ///
    /// ## How It Works
    /// The TTL algorithm pairs time-based expiration with insertion-order eviction. Every entry
    /// is stamped with an absolute `expires_at` timestamp at insertion time (computed as
    /// `now + global_ttl`). Entries are stored in insertion order, and eviction proceeds from the
    /// front of that queue — but only after confirming the candidate has actually expired. A live
    /// entry at the front of the queue blocks eviction of everything behind it, so the cache may
    /// temporarily exceed capacity if the oldest entries are still fresh.
    ///
    /// Like `FIFOPolicy`, this implementation backs the queue with a `double-ended queue` for O(1)
    /// front removal and a `hash map` for O(1) key lookups. The same logical-index trick applies:
    /// the table stores monotonically increasing counters rather than physical deque positions, and
    /// a `front_offset` counter converts a logical index back to a physical one at read time via
    /// `entries[table[key] - front_offset]`. This keeps eviction and lookup O(1) without rewriting
    /// the table on every eviction. On top of that, every read checks `expires_at` against the current wall-clock time and
    /// treats any expired entry as a cache miss.
    ///
    /// Without `sweep_interval`, an expiry sweep is triggered automatically on every call to
    /// `insert`, `update`, `current_size`, `remaining_size`, `last`, `first`, `items`, `keys`,
    /// `values`, and `__iter__`. A completely idle cache will accumulate stale entries between
    /// these calls, but any normal interaction with the cache is sufficient to reclaim them.
    /// When `sweep_interval` is set, a background Rust thread performs the sweep on that interval
    /// instead, reclaiming expired entries independent of any method calls.
    ///
    /// ### Pros
    /// - Insert, lookup, and evict are all O(1) amortized: the `front_offset` trick eliminates the O(n)
    ///   index-shifting that a naive implementation would require on every eviction.
    /// - Entries expire automatically without any background thread or explicit invalidation call.
    ///   Stale data is never returned to the caller.
    /// - TTL expiry and insertion-order eviction compose cleanly: the oldest entry is always evicted
    ///   first among those that have already expired.
    /// - A single `global_ttl` keeps configuration simple; every entry ages at the same rate.
    ///
    /// ### Cons
    ///
    /// - Wall-clock dependency. Correctness relies on a monotonically advancing system clock.
    ///   Clock adjustments (NTP steps, suspend/resume) can cause entries to expire earlier or later
    ///   than intended.
    /// - When `sweep_interval` is set, a background thread wakes on that interval to sweep and
    ///   remove all expired entries. This adds a small amount of background CPU usage and
    ///   introduces a reaper thread for the lifetime of the cache.
    /// - No per-entry TTL override. All entries share `global_ttl`; mixed expiry requirements need
    ///   a different policy or a wrapper layer.
    /// - The rare O(n) index rebase (triggered when `front_offset` nears `usize::MAX - isize::MAX`)
    ///   introduces an occasional latency spike. Amortized cost is negligible, but worst-case
    ///   latency is unbounded in principle.
    ///
    /// ## When to use it
    /// Reach for `TTLPolicy` when:
    /// - Cached data has a natural freshness window: API responses, auth tokens, DNS records,
    ///   rate-limit counters, or any value that becomes incorrect or unsafe after a known interval.
    /// - You need automatic expiry without a background reaper thread — expiry sweeps on common
    ///   method calls are sufficient, or you want continuous reclamation via `sweep_interval`.
    /// - Access patterns are unpredictable or uniform enough that recency- or frequency-based
    ///   eviction (LRU/LFU) would offer no meaningful advantage.
    ///
    /// Avoid it when:
    /// - Your workload has strong temporal locality and you need a best-effort hit rate policy —
    ///   LRU will serve you better.
    /// - Per-entry TTL granularity is required. If different keys need different lifetimes,
    ///   consider a policy that accepts per-insertion expiry hints.
    /// - Your environment has an unreliable or adjustable system clock, where wall-clock-based
    ///   expiry may behave unexpectedly.
    [subclass, extends=crate::pyclasses::base::PyBaseCacheImpl, generic, frozen]
    PyTTLCache as "TTLCache" (onceinit::OnceInit<Wrapped<ttlpolicy::TTLPolicy>>);
}

#[pyo3::pymethods]
impl PyTTLCache {
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

    /// Initialize a new `PyTTLCache` instance.
    ///
    /// Args:
    ///     maxsize: Maximum number of elements the cache can hold.
    ///     global_ttl: Time-to-live for cache entries, either as seconds or a timedelta.
    ///     iterable: Initial data to populate the cache.
    ///     capacity: Pre-allocate capacity to minimize reallocations. Defaults to 0.
    ///     getsizeof: A callable that computes the size of a key-value pair. When `None`, each
    ///             entry is assumed to have a size of 1 (equivalent to `lambda k, v: 1`).
    ///             Use this to implement weighted caching — for example, sizing entries by
    ///             memory footprint or byte length.
    ///
    /// The cache can be pre-sized via `capacity` to reduce hash table reallocations when
    /// the number of expected entries is known ahead of time.
    #[pyo3(signature=(maxsize, global_ttl, iterable=None, *, capacity=0, getsizeof=None))]
    fn __init__(
        &self,
        py: pyo3::Python,
        maxsize: usize,
        global_ttl: utils::FloatOrTimedelta,
        iterable: Option<alias::BoundObject>,
        capacity: usize,
        getsizeof: Option<alias::PyObject>,
    ) -> pyo3::PyResult<()> {
        let global_ttl: f64 = global_ttl.into();
        if global_ttl <= 0.0 {
            return Err(new_py_error!(
                PyValueError,
                "global_ttl must be positive and non-zero"
            ));
        }

        let wrapped = Wrapped::new(ttlpolicy::TTLPolicy::new(capacity), unsafe {
            ttlpolicy::Shared::with_ttl(maxsize, getsizeof, Some(global_ttl))
        });

        // Populate cache if `iterable` passed
        let extend_result = {
            if let Some(iterable) = iterable {
                let ttl: ttlpolicy::ExpiresAt = wrapped.shared().global_ttl().unwrap().into();
                let getsizeof = wrapped.shared().getsizeof().clone_ref(py);

                let result = wrapped.extend(
                    // iterable object
                    iterable,
                    // transform function
                    |key, value| ttlpolicy::ExpiringHandle::new(py, &getsizeof, ttl, key, value),
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
    fn current_size(&self) -> pyo3::PyResult<usize> {
        let inner = self.0.get();
        let mut policy = inner.policy();
        policy.expire(inner.shared().generation_version())?;
        Ok(policy.current_size())
    }

    #[inline]
    fn remaining_size(&self) -> pyo3::PyResult<usize> {
        let inner = self.0.get();
        {
            let mut policy = inner.policy();
            policy.expire(inner.shared().generation_version())?;
        }

        Ok(inner.remaining_size())
    }

    #[getter]
    #[inline]
    fn getsizeof(&self, py: pyo3::Python) -> Option<alias::PyObject> {
        let inner = self.0.get();
        inner.shared().getsizeof().clone_ref(py).into()
    }

    #[getter]
    #[inline]
    fn global_ttl(&self) -> f64 {
        let inner = self.0.get();
        unsafe { inner.shared().global_ttl().unwrap_unchecked().as_secs_f64() }
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    fn capacity(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        policy.table().capacity().min(policy.entries().capacity())
    }

    /// Returns the number of entries currently in the cache.
    #[inline]
    fn __len__(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        debug_assert!(policy.table().len() == policy.entries().len());
        policy.table().len()
    }

    #[inline]
    fn __sizeof__(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        let table_cap = policy.table().capacity() * std::mem::size_of::<usize>();
        let vecdeque_cap =
            policy.entries().capacity() * std::mem::size_of::<ttlpolicy::ExpiringHandle>();

        table_cap + vecdeque_cap
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
    fn insert(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Option<alias::PyObject>> {
        let inner = self.0.get();
        let shared = inner.shared();
        let handle = ttlpolicy::ExpiringHandle::new(
            py,
            shared.getsizeof(),
            unsafe { shared.global_ttl().unwrap_unchecked().into() },
            key,
            value,
        )?;

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
        let shared = inner.shared();

        let ttl: ttlpolicy::ExpiresAt = unsafe { shared.global_ttl().unwrap_unchecked().into() };
        let getsizeof = shared.getsizeof().clone_ref(py);

        inner.extend(
            // iterable object
            iterable.into_bound(py),
            // transform function
            move |key, value| ttlpolicy::ExpiringHandle::new(py, &getsizeof, ttl, key, value),
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

        let handle = ttlpolicy::ExpiringHandle::with_precomputed_hash_key(
            py,
            shared.getsizeof(),
            unsafe { shared.global_ttl().unwrap_unchecked().into() },
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

    fn items(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyTTLCacheItems>> {
        let inner = self.0.get();

        let iter = inner.policy().iter(inner.shared())?;

        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyTTLCacheItems {
            iter: parking_lot::Mutex::new(iter),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    fn values(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyTTLCacheValues>> {
        let inner = self.0.get();

        let iter = inner.policy().iter(inner.shared())?;

        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyTTLCacheValues {
            iter: parking_lot::Mutex::new(iter),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    fn keys(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyTTLCacheKeys>> {
        let inner = self.0.get();

        let iter = inner.policy().iter(inner.shared())?;

        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyTTLCacheKeys {
            iter: parking_lot::Mutex::new(iter),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    #[inline]
    fn __iter__(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyTTLCacheKeys>> {
        self.keys(py)
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

    fn __repr__(slf: pyo3::PyRef<'_, Self>, py: pyo3::Python) -> String {
        let inner = slf.0.get();
        let shared = inner.shared();
        let policy = inner.policy();

        let now = std::time::SystemTime::now();
        let iter = policy
            .entries()
            .iter()
            .filter(|handle| !handle.is_expired(now))
            .map(|handle| {
                (
                    // Without using `.bind` it returns something like `Py(addr)`
                    handle.key().as_ref().bind(py),
                    handle.value().bind(py),
                )
            });

        let items = utils::items_to_str(iter, policy.table().len()).unwrap();
        format!(
            "{}[{}/{}]({})",
            unsafe { utils::get_type_name(py, slf.as_ptr()) },
            policy.current_size(),
            shared.maxsize(),
            items
        )
    }

    #[inline]
    #[pyo3(signature=(*, reuse=false))]
    fn expire(&self, reuse: bool) -> pyo3::PyResult<()> {
        let inner = self.0.get();
        let shared = inner.shared();
        let mut policy = inner.policy();

        policy.expire(shared.generation_version())?;

        if !reuse {
            policy.shrink_to_fit(shared);
        }
        Ok(())
    }

    #[pyo3(signature = (n=0))]
    fn first(
        &self,
        py: pyo3::Python,
        mut n: pyo3::ffi::Py_ssize_t,
    ) -> pyo3::PyResult<alias::PyObject> {
        let inner = self.0.get();
        let mut policy = inner.policy();

        policy.expire(inner.shared().generation_version())?;

        if n < 0 {
            n += policy.entries().len() as isize;
        }
        if n < 0 {
            return Err(new_py_error!(PyIndexError, "`n` out of range"));
        }

        match policy.entries().get(n as usize) {
            Some(handle) => Ok(handle.key().as_ref().clone_ref(py)),
            None => Err(new_py_error!(PyIndexError, "`n` out of range")),
        }
    }

    fn last(&self, py: pyo3::Python) -> pyo3::PyResult<alias::PyObject> {
        let inner = self.0.get();
        let mut policy = inner.policy();

        policy.expire(inner.shared().generation_version())?;

        match policy.entries().back() {
            Some(handle) => Ok(handle.key().as_ref().clone_ref(py)),
            None => Err(new_py_error!(PyIndexError, "`n` out of range")),
        }
    }

    #[pyo3(signature = (key, default=utils::OptionalArgument::Undefined))]
    fn get_with_expire(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        default: utils::OptionalArgument,
    ) -> pyo3::PyResult<(alias::PyObject, f64)> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();
        let mut policy = inner.policy();

        if let Some(x) = policy.get(py, &key)? {
            let dur = x
                .expires_at()
                .duration_since(std::time::SystemTime::now())
                .unwrap_or_default();

            return Ok((x.value().clone_ref(py), dur.as_secs_f64()));
        }

        match default {
            utils::OptionalArgument::Defined(x) => Ok((x, 0.0)),
            utils::OptionalArgument::Undefined => unsafe {
                // SAFETY: None is immortal, so reference counting has no meaning
                Ok((
                    pyo3::Bound::from_owned_ptr(py, pyo3::ffi::Py_None()).unbind(),
                    0.0,
                ))
            },
        }
    }

    #[pyo3(signature = (key, default=utils::OptionalArgument::Undefined))]
    fn pop_with_expire(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        default: utils::OptionalArgument,
    ) -> pyo3::PyResult<(alias::PyObject, f64)> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();

        if let Some(x) = inner.remove(py, &key)? {
            let dur = x
                .expires_at()
                .duration_since(std::time::SystemTime::now())
                .unwrap_or_default();

            return Ok((x.into_value(), dur.as_secs_f64()));
        }

        match default {
            utils::OptionalArgument::Defined(x) => Ok((x, 0.0)),
            utils::OptionalArgument::Undefined => Err(new_py_error!(
                PyKeyError,
                Into::<alias::PyObject>::into(key)
            )),
        }
    }

    fn popitem_with_expire(&self) -> pyo3::PyResult<(alias::PyObject, alias::PyObject, f64)> {
        let inner = self.0.get();
        let mut policy = inner.policy();

        let handle = policy.evict(inner.shared())?;
        drop(policy);

        let dur = handle
            .expires_at()
            .duration_since(std::time::SystemTime::now())
            .unwrap_or_default();

        let (key, val) = handle.into_pair();
        Ok((key.into(), val, dur.as_secs_f64()))
    }

    fn items_with_expire(
        &self,
        py: pyo3::Python,
    ) -> pyo3::PyResult<pyo3::Py<PyTTLCacheItemsWithExpire>> {
        let inner = self.0.get();

        let iter = inner.policy().iter(inner.shared())?;

        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyTTLCacheItemsWithExpire {
            iter: parking_lot::Mutex::new(iter),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        let inner = self.0.get();
        let policy = inner.policy();

        for handle in policy.entries().iter() {
            visit.call(handle.key().as_ref())?;
            visit.call(handle.value())?;
        }
        Ok(())
    }

    fn __clear__(&self) {
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
                [extends=crate::pyclasses::base::PyBaseIteratorImpl, generic, frozen]
                $name as $pyname {
                    initial_gv: u32,
                    gv: utils::GenerationVersion,
                    iter: parking_lot::Mutex<utils::RawVecDequeIter<ttlpolicy::ExpiringHandle>>,
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

                    let now = std::time::SystemTime::now();
                    let mut iter = slf.iter.lock();
                    let $py = slf.py();

                    while let Some(x) = iter.next() {
                        let $handle = unsafe { x.as_ref() };
                        if $handle.is_expired(now) {
                            continue;
                        }

                        return Ok($init);
                    }

                    Err(new_py_error!(PyStopIteration, ()))
                }
            }
        )+
    };
}
implement_iterator!(
    PyTTLCacheItems as "ttlcache_items"
    fn(py, handle) -> (alias::PyObject, alias::PyObject) {{
        let (key, val) = handle.clone_ref(py).into_pair();
        (key.into(), val)
    }}

    PyTTLCacheItemsWithExpire as "ttlcache_items_with_expire"
    fn(py, handle) -> (alias::PyObject, alias::PyObject, f64) {{
        let dur = handle
            .expires_at()
            .duration_since(std::time::SystemTime::now())
            .unwrap_or_default();

        let (key, val) = handle.clone_ref(py).into_pair();
        (key.into(), val, dur.as_secs_f64())
    }}

    PyTTLCacheKeys as "ttlcache_keys"
    fn(py, handle) -> alias::PyObject { handle.key().clone_ref(py).into() }

    PyTTLCacheValues as "ttlcache_values"
    fn(py, handle) -> alias::PyObject { handle.value().clone_ref(py) }
);
