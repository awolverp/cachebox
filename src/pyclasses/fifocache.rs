use crate::internal::alias;
use crate::internal::onceinit;
use crate::internal::utils;
use crate::policies::fifopolicy;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;
use crate::policies::wrapped::Wrapped;

implement_pyclass! {
    /// A First-In-First-Out (FIFO) cache eviction policy: when the cache is full, the oldest
    /// inserted item is always the first to be removed, regardless of how often it has been accessed.
    ///
    /// ## How It Works
    /// The FIFO algorithm is one of the simplest cache eviction strategies. Items are stored in
    /// insertion order, and when the cache reaches capacity, the item that has been there the
    /// longest is evicted to make room. There is no concept of "recently used" or "frequently used"
    /// - age alone determines eviction order. Conceptually, it behaves like a queue: new items
    /// join the back, and evictions come from the front.
    ///
    /// This implementation backs that queue with a `double-ended queue` for O(1) front removal,
    /// paired with a `hash map` for O(1) key lookups. Rather than storing physical indices into
    /// the deque (which shift every time an item is evicted from the front), the table stores
    /// logical indices - a monotonically increasing counter assigned at insertion time.
    /// A separate `front_offset` counter tracks how many items have ever been evicted; the physical
    /// position of any key is recovered at read time as `entries[table[key] - front_offset]`,
    /// keeping both eviction and lookup O(1) without any per-eviction rewriting of the table.
    ///
    /// ### Pros
    /// - Insert, lookup, and evict are all O(1) amortized: the `front_offset` trick eliminates the O(n)
    ///   index-shifting that a native implementation would require on every eviction.
    /// - Eviction order is fully deterministic: the oldest item always goes first, independent of access
    ///   patterns, making behaviour easy to reason about and reproduce in tests.
    /// - No per-read overhead. Unlike LRU, FIFO requires no bookkeeping on cache hits.
    ///
    /// ### Cons
    /// - Access-blind eviction. A hot item accessed thousands of times is evicted just as readily as one
    ///   that has never been read. Hit rates suffer on workloads with strong temporal locality.
    /// - The logical-index indirection adds a layer of internal complexity compared to a naïve queue-based cache.
    /// - The rare O(n) index rebase (triggered when `front_offset` nears `usize::MAX - isize::MAX`) introduces
    ///   an occasional latency spike. Amortized cost is negligible, but worst-case latency is unbounded in principle.
    ///
    /// ## When to use it
    /// Reach for `FIFOPolicy` when:
    /// - Eviction order must be predictable and auditable: streaming pipelines, sequential batch processors, or
    ///   any context where deterministic behaviour simplifies debugging.
    /// - Access patterns are roughly uniform, so there is no meaningful "hot" subset of keys that a recency or
    ///   frequency-aware policy could exploit.
    /// - Read overhead must be minimal: FIFO's zero-cost hits make it preferable to LRU in insert-heavy workloads
    ///   with infrequent re-reads.
    ///
    /// Avoid it when your workload has strong temporal locality. If recently or frequently accessed items are likely
    /// to be needed again soon, an LRU or LFU policy will deliver meaningfully better hit rates.
    [subclass, extends=crate::pyclasses::base::PyBaseCacheImpl, generic, frozen]
    PyFIFOCache as "FIFOCache" (onceinit::OnceInit<Wrapped<fifopolicy::FIFOPolicy>>);
}

#[pyo3::pymethods]
impl PyFIFOCache {
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

    /// Initialize a new `FIFOCache` instance.
    ///
    /// Args:
    ///     maxsize: Maximum number of elements the cache can hold.
    ///     iterable: Initial data to populate the cache.
    ///     capacity: Pre-allocate capacity to minimize reallocations. Defaults to 0.
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
            fifopolicy::FIFOPolicy::new(capacity),
            fifopolicy::Shared::new(maxsize, getsizeof),
        );

        if let Some(iterable) = iterable {
            let getsizeof = wrapped.shared().getsizeof().clone_ref(py);

            let result = wrapped.extend(
                // iterable object
                iterable,
                // transform function
                |key, value| fifopolicy::Handle::new(py, &getsizeof, key, value),
            );
            self.0.set(wrapped);
            result
        } else {
            self.0.set(wrapped);
            Ok(())
        }
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
        let vecdeque_cap = policy.entries().capacity() * std::mem::size_of::<fifopolicy::Handle>();
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
        let handle = fifopolicy::Handle::new(py, inner.shared().getsizeof(), key, value)?;

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
            move |key, value| fifopolicy::Handle::new(py, &getsizeof, key, value),
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
    fn get<'p>(
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

        let handle = fifopolicy::Handle::with_precomputed_hash_key(
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
    fn popitem(&self, py: pyo3::Python) -> pyo3::PyResult<(alias::PyObject, alias::PyObject)> {
        let inner = self.0.get();
        let mut policy = inner.policy();

        let handle = policy.evict(py, inner.shared())?;
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

    fn items(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyFIFOCacheItems>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyFIFOCacheItems {
            iter: parking_lot::Mutex::new(inner.policy().iter()),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    fn values(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyFIFOCacheValues>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyFIFOCacheValues {
            iter: parking_lot::Mutex::new(inner.policy().iter()),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    fn keys(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyFIFOCacheKeys>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyFIFOCacheKeys {
            iter: parking_lot::Mutex::new(inner.policy().iter()),
            gv,
            initial_gv,
        };
        pyo3::Py::new(py, (result, crate::pyclasses::base::PyBaseIteratorImpl))
    }

    #[inline]
    fn __iter__(&self, py: pyo3::Python) -> pyo3::PyResult<pyo3::Py<PyFIFOCacheKeys>> {
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

        let iter = policy.entries().iter().map(|handle| {
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

    #[pyo3(signature = (n=0))]
    fn first(
        &self,
        py: pyo3::Python,
        mut n: pyo3::ffi::Py_ssize_t,
    ) -> pyo3::PyResult<alias::PyObject> {
        let inner = self.0.get();
        let policy = inner.policy();

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
        let policy = inner.policy();
        match policy.entries().back() {
            Some(handle) => Ok(handle.key().as_ref().clone_ref(py)),
            None => Err(new_py_error!(PyIndexError, "`n` out of range")),
        }
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
                    iter: parking_lot::Mutex<utils::RawVecDequeIter<fifopolicy::Handle>>,
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
    PyFIFOCacheItems as "fifocache_items"
    fn(py, handle) -> (alias::PyObject, alias::PyObject) {{
        let (key, val) = handle.clone_ref(py).into_pair();
        (key.into(), val)
    }}

    PyFIFOCacheKeys as "fifocache_keys"
    fn(py, handle) -> alias::PyObject { handle.key().clone_ref(py).into() }

    PyFIFOCacheValues as "fifocache_values"
    fn(py, handle) -> alias::PyObject { handle.value().clone_ref(py) }
);
