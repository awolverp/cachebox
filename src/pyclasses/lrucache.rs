use crate::internal::alias;
use crate::internal::linked_list;
use crate::internal::onceinit;
use crate::internal::utils;
use crate::policies::lrupolicy;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;
use crate::policies::wrapped::Wrapped;

implement_pyclass! {
    /// A Least-Recently-Used (LRU) cache eviction policy: when the cache is full,
    /// the item that has not been accessed for the longest time is removed first,
    /// regardless of how many times it was accessed in the past.
    ///
    /// ## How It Works
    /// The LRU algorithm is one of the most widely used cache eviction strategies in
    /// practice. Items are tracked by their access recency—every time an item is read
    /// or written, it becomes the most recently used. When the cache reaches capacity,
    /// the least recently used item (the one that was accessed longest ago) is
    /// evicted to make room for new entries.
    ///
    /// This implementation pairs a doubly-linked list with a hash map. The linked list
    /// maintains items in access order: the most recently used item sits at the back,
    /// and the least recently used at the front. The hash map stores pointers (cursors)
    /// into this list, enabling O(1) key lookups. On every access—read or write—the
    /// accessed item is moved to the back of the list, promoting it to "most recently used"
    /// status. When eviction is needed, the front item is removed.
    ///
    /// The doubly-linked list structure is critical: it permits O(1) removal and
    /// reinsertion of any item anywhere in the ordering, without requiring a full rebuild
    /// or index shifting. A running total tracks the current size of cached items,
    /// allowing capacity checks in constant time.
    ///
    /// ### Pros
    /// - **Excellent hit rates on temporal locality.** Workloads where recently or
    ///   frequently accessed items are likely to be needed again soon benefit dramatically
    ///   from LRU's recency-aware eviction. Real-world caches (CPU L1/L2, database
    ///   buffers, CDN edges) rely on this principle.
    /// - **Insert, lookup, and evict are all O(1) amortized.** The doubly-linked list
    ///   and hash map combination guarantees no per-operation index shifting or traversals.
    /// - **Automatic adaptation to access patterns.** Hot keys naturally migrate to the
    ///   back of the list and stay there, while cold keys drift toward eviction. No
    ///   manual tuning of weights or thresholds is needed.
    /// - **Per-hit cost is minimal.** While LRU does require bookkeeping on reads (moving
    ///   an item to the back), this bookkeeping is O(1) and adds negligible overhead to most
    ///   workloads.
    ///
    /// ### Cons
    /// - **Per-read overhead.** Every cache hit requires updating the linked list (removing
    ///   the item from its current position and reinserting it at the back), which is
    ///   measurably slower than FIFO's zero-cost hits on read-heavy workloads.
    /// - **Burst traffic can skew eviction.** A single item accessed many times in rapid
    ///   succession will be kept alive indefinitely, even if other keys have better long-term
    ///   utility. Recency is a proxy for future use, not a guarantee.
    /// - **Implementation complexity.** The doubly-linked list and cursor-based hash table add
    ///   internal complexity compared to simpler policies like FIFO.
    /// - **Memory overhead.** Storing doubly-linked pointers (prev/next) for every cached item
    ///   consumes extra memory compared to array-based alternatives.
    ///
    /// ## When to use it
    /// Reach for `LRUPolicy` when:
    /// - Your workload exhibits temporal locality—recently accessed items are likely to be
    ///   needed again soon. Databases, web caches, and CPU caches all exhibit this pattern.
    /// - Hit rate is your primary metric. If maximizing the proportion of requests served
    ///   from the cache matters more than minimizing per-hit latency, LRU is typically the
    ///   best general-purpose choice.
    /// - Access patterns are unknown or unpredictable. LRU's automatic adaptation makes it a safe
    ///   default when you cannot statically analyze what keys will be hot.
    /// - You need a standard, battle-tested algorithm. LRU is the de facto eviction policy in most
    ///   production systems; it is well-understood, widely supported, and easy to reason about.
    ///
    /// Avoid it when:
    /// - Your workload is write-heavy with few or no re-reads. FIFO's zero per-hit bookkeeping
    ///   will outperform LRU if the cache is rarely hit.
    /// - You need sub-microsecond latency on every operation. The linked-list manipulation on each
    ///   read can add measurable overhead in ultra-low-latency systems.
    /// - Access patterns are bimodal or exhibit frequency-heavy behavior (a small set of items is
    ///   accessed far more often than others). An LFU policy may deliver better hit rates in such cases.
    [subclass, extends=crate::pyclasses::base::PyBaseCacheImpl, generic, frozen]
    PyLRUCache as "LRUCache" (onceinit::OnceInit<Wrapped<lrupolicy::LRUPolicy>>);
}

#[pyo3::pymethods]
impl PyLRUCache {
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

    /// Initialize a new `LRUCache` instance.
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
            lrupolicy::LRUPolicy::new(capacity),
            lrupolicy::Shared::new(maxsize, getsizeof),
        );

        // Populate cache if `iterable` passed
        let extend_result = {
            if let Some(iterable) = iterable {
                let getsizeof = wrapped.shared().getsizeof().clone_ref(py);

                let result = wrapped.extend(
                    // iterable object
                    iterable,
                    // transform function
                    |key, value| lrupolicy::Handle::new(py, &getsizeof, key, value),
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

        debug_assert!(policy.table().len() == policy.list().len());
        policy.table().len()
    }

    #[inline]
    fn __sizeof__(&self) -> usize {
        let inner = self.0.get();
        let policy = inner.policy();

        let table_cap = policy.table().capacity() * 8;
        let list_cap = policy.list().len() * std::mem::size_of::<lrupolicy::Handle>();

        table_cap + list_cap
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
        let handle = lrupolicy::Handle::new(py, inner.shared().getsizeof(), key, value)?;

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
            move |key, value| lrupolicy::Handle::new(py, &getsizeof, key, value),
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

        let handle = lrupolicy::Handle::with_precomputed_hash_key(
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

    fn items(&self) -> pyo3::PyResult<pyo3::Py<PyLRUCacheItems>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyLRUCacheItems {
            iter: parking_lot::Mutex::new(unsafe { inner.policy().list().iter() }),
            gv,
            initial_gv,
        };
        pyo3::Python::attach(|py| pyo3::Py::new(py, result))
    }

    fn values(&self) -> pyo3::PyResult<pyo3::Py<PyLRUCacheValues>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyLRUCacheValues {
            iter: parking_lot::Mutex::new(unsafe { inner.policy().list().iter() }),
            gv,
            initial_gv,
        };
        pyo3::Python::attach(|py| pyo3::Py::new(py, result))
    }

    fn keys(&self) -> pyo3::PyResult<pyo3::Py<PyLRUCacheKeys>> {
        let inner = self.0.get();
        let gv = inner.shared().generation_version().clone();
        let initial_gv = gv.get();

        // SAFETY: We cannot use lifetimes here, but we're tracking changes using [`GenerationVersion`]
        let result = PyLRUCacheKeys {
            iter: parking_lot::Mutex::new(unsafe { inner.policy().list().iter() }),
            gv,
            initial_gv,
        };
        pyo3::Python::attach(|py| pyo3::Py::new(py, result))
    }

    #[inline]
    fn __iter__(&self) -> pyo3::PyResult<pyo3::Py<PyLRUCacheKeys>> {
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
            policy.list().iter().map(|cursor| {
                let handle = cursor.element();
                (
                    // Without `.bind` it returns something like `Py(addr)`
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

    #[pyo3(signature = (key, default=utils::OptionalArgument::Undefined))]
    fn peek(
        &self,
        py: pyo3::Python,
        key: alias::PyObject,
        default: utils::OptionalArgument,
    ) -> pyo3::PyResult<alias::PyObject> {
        let key = utils::PrecomputedHashObject::new(py, key)?;

        let inner = self.0.get();
        let policy = inner.policy();

        if let Some(x) = policy.peek(py, &key)? {
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

    #[inline]
    fn least_recently_used(&self, py: pyo3::Python) -> pyo3::PyResult<alias::PyObject> {
        let inner = self.0.get();
        let policy = inner.policy();

        match policy.list().cursor_front() {
            Some(cursor) => Ok(unsafe { cursor.element().key().clone_ref(py).into() }),
            None => Err(new_py_error!(PyKeyError, "cache is empty")),
        }
    }

    #[inline]
    fn most_recently_used(&self, py: pyo3::Python) -> pyo3::PyResult<alias::PyObject> {
        let inner = self.0.get();
        let policy = inner.policy();

        match policy.list().cursor_back() {
            Some(cursor) => Ok(unsafe { cursor.element().key().clone_ref(py).into() }),
            None => Err(new_py_error!(PyKeyError, "cache is empty")),
        }
    }

    fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        if self.0.is_initialized() {
            return Ok(());
        }

        let inner = self.0.get();
        let policy = inner.policy();

        for cursor in unsafe { policy.list().iter() } {
            let handle = unsafe { cursor.element() };

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
                    iter: parking_lot::Mutex<linked_list::RawIter<lrupolicy::Handle>>,
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
                            let $handle = unsafe { x.element() };
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
    PyLRUCacheItems as "lrucache_items"
    fn(py, handle) -> (alias::PyObject, alias::PyObject) {{
        let (key, val) = handle.clone_ref(py).into_pair();
        (key.into(), val)
    }}

    PyLRUCacheKeys as "lrucache_keys"
    fn(py, handle) -> alias::PyObject { handle.key().clone_ref(py).into() }

    PyLRUCacheValues as "lrucache_values"
    fn(py, handle) -> alias::PyObject { handle.value().clone_ref(py) }
);
