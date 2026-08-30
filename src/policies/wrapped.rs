use std::collections::VecDeque;

use pyo3::types::PyAnyMethods;
use pyo3::types::PyTupleMethods;

use crate::internal::alias;
use crate::internal::pickle;
use crate::internal::pickle::Builder;
use crate::policies::traits::HandleExt;
use crate::policies::traits::OccupiedExt;
use crate::policies::traits::PolicyEntry;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;
use crate::policies::traits::VacantExt;

/// A wrapper over [`PolicyExt`] implementations that adds
/// higher-level methods shared across all policies.
///
/// - [`insert`](Wrapped::insert)
/// - [`remove`](Wrapped::remove)
/// - [`contains`](Wrapped::contains)
/// - [`extend`](Wrapped::extend).
///
/// The shared (lock-free) fields of the policy are accessible directly via
/// [`Wrapped::shared`], while mutable state is accessed through the inner
/// [`std::sync::Mutex`].
pub struct Wrapped<P: PolicyExt> {
    /// Read-only fields after initialization — no lock required.
    /// Accessible directly without acquiring the mutex.
    shared: P::Shared,
    /// Mutable policy state — protected by a [`std::sync::Mutex`].
    inner: parking_lot::Mutex<P>,
}

/// A lock guard that defers the destruction of removed values.
///
/// Handles removed by internal operations are parked in the policy's
/// [`PolicyExt::pending_drops`] buffer. When this guard goes out of scope it
/// first releases the mutex and only then drops those handles, so Python code
/// running in a value's ``__del__`` never executes while the lock is held.
pub struct PolicyGuard<'a, P: PolicyExt> {
    guard: std::mem::ManuallyDrop<parking_lot::MutexGuard<'a, P>>,
}

impl<'a, P: PolicyExt> PolicyGuard<'a, P> {
    #[inline(always)]
    fn new(guard: parking_lot::MutexGuard<'a, P>) -> Self {
        Self {
            guard: std::mem::ManuallyDrop::new(guard),
        }
    }
}

impl<'a, P: PolicyExt> std::ops::Deref for PolicyGuard<'a, P> {
    type Target = P;

    #[inline(always)]
    fn deref(&self) -> &P {
        &self.guard
    }
}

impl<'a, P: PolicyExt> std::ops::DerefMut for PolicyGuard<'a, P> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut P {
        &mut self.guard
    }
}

impl<'a, P: PolicyExt> Drop for PolicyGuard<'a, P> {
    #[inline]
    fn drop(&mut self) {
        let buffer = self.guard.pending_drops();

        if buffer.is_empty() {
            // SAFETY: the guard is dropped exactly once per branch.
            unsafe { std::mem::ManuallyDrop::drop(&mut self.guard) };
            return;
        }

        if buffer.len() == 1 {
            // `pop` keeps the buffer's allocation for the next eviction.
            let handle = buffer.pop();
            // SAFETY: the guard is dropped exactly once per branch.
            unsafe { std::mem::ManuallyDrop::drop(&mut self.guard) };
            // The lock is released: the destructor may run Python code.
            drop(handle);
            return;
        }

        let pending = std::mem::take(buffer);
        // SAFETY: the guard is dropped exactly once per branch.
        unsafe { std::mem::ManuallyDrop::drop(&mut self.guard) };
        // The lock is released: the destructors may run Python code.
        drop(pending);
    }
}

impl<P: PolicyExt> Wrapped<P> {
    /// Wraps an existing policy alongside its shared (lock-free) data.
    pub fn new(policy: P, shared: P::Shared) -> Self {
        Self {
            shared,
            inner: parking_lot::Mutex::new(policy),
        }
    }

    /// Returns a reference to the shared, lock-free fields of the policy.
    #[inline(always)]
    pub fn shared(&self) -> &P::Shared {
        &self.shared
    }

    /// Acquires the mutex and returns a guard over the mutable policy state.
    ///
    /// # Panics
    /// Panics if the mutex is poisoned.
    #[inline(always)]
    pub fn policy(&self) -> PolicyGuard<'_, P> {
        PolicyGuard::new(self.inner.lock())
    }

    /// Acquires the mutex only if it is free, returning `None` otherwise.
    ///
    /// For callers that must never wait for the lock, such as `__traverse__`:
    /// the thread holding the lock may be running Python code, and a garbage
    /// collection pass landing there would deadlock the whole process.
    #[inline(always)]
    pub fn try_policy(&self) -> Option<PolicyGuard<'_, P>> {
        self.inner.try_lock().map(PolicyGuard::new)
    }
}

#[inline(always)]
fn insert_inner<P: PolicyExt>(
    lock: &mut PolicyGuard<'_, P>,
    shared: &P::Shared,
    py: pyo3::Python<'_>,
    handle: P::Handle,
) -> pyo3::PyResult<Option<P::Handle>> {
    match insert_attempt(lock, shared, py, handle) {
        Ok(result) => Ok(result),
        Err((err, rejected)) => {
            if let Some(handle) = rejected {
                lock.pending_drops().push(handle);
            }
            Err(err)
        }
    }
}

/// The locked part of [`insert_inner`]. On failure the handle that did not
/// make it into the cache is handed back, so the caller can park it.
#[inline(always)]
fn insert_attempt<P: PolicyExt>(
    lock: &mut PolicyGuard<'_, P>,
    shared: &P::Shared,
    py: pyo3::Python<'_>,
    handle: P::Handle,
) -> Result<Option<P::Handle>, (pyo3::PyErr, Option<P::Handle>)> {
    let handle_size = handle.size();

    if handle_size > shared.maxsize() {
        let err = new_py_error!(
            PyOverflowError,
            "handle size is more than the configured maximum size"
        );
        return Err((err, Some(handle)));
    }

    let mut result = match lock.entry(py, handle.key(), shared) {
        Err(err) => return Err((err, Some(handle))),
        Ok(PolicyEntry::Occupied(occupied)) => Some(occupied.replace(handle)),
        Ok(PolicyEntry::Vacant(mut vacant)) => {
            // Evict if need
            let mut eviction_failed = None;
            while vacant.would_exceed(handle_size) {
                if let Err(err) = vacant.evict() {
                    eviction_failed = Some(err);
                    break;
                }
            }

            if let Some(err) = eviction_failed {
                drop(vacant);
                return Err((err, Some(handle)));
            }

            vacant.insert(handle);
            None
        }
    };

    if result.is_some() {
        // For the `PolicyEntry::Occupied` case, evict after replacement
        while lock.current_size() > shared.maxsize() {
            match lock.evict(shared) {
                Ok(evicted) => lock.pending_drops().push(evicted),
                Err(err) => return Err((err, result.take())),
            }
        }
    }

    Ok(result)
}

// Duplicate methods across all policies
impl<P: PolicyExt> Wrapped<P> {
    /// Returns the remaining size. Equals to `maxsize - current_size`.
    #[inline]
    pub fn remaining_size(&self) -> usize {
        let policy = self.policy();
        self.shared.maxsize().saturating_sub(policy.current_size())
    }

    /// Returns `true` if the cache contains an entry for `key`.
    #[inline]
    pub fn contains(
        &self,
        py: pyo3::Python<'_>,
        key: &<P::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<bool> {
        let mut lock = self.policy();

        let handle = lock.get(py, key, &self.shared)?;
        Ok(handle.is_some())
    }

    /// Inserts a [`Handle`](PolicyExt::Handle) into the cache, evicting entries as needed
    /// to stay within the size budget before inserting.
    ///
    /// - If the key was already present, the old handle is replaced and returned as `Some`.
    /// - If the key was absent, the handle is inserted and `None` is returned.
    #[inline]
    pub fn insert_no_lock(
        &self,
        policy: &mut PolicyGuard<'_, P>,
        py: pyo3::Python<'_>,
        handle: P::Handle,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        insert_inner(policy, &self.shared, py, handle)
    }

    /// See [Self::insert_no_lock]
    #[inline]
    pub fn insert(
        &self,
        py: pyo3::Python<'_>,
        handle: P::Handle,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        let mut lock = self.policy();
        self.insert_no_lock(&mut lock, py, handle)
    }

    /// Removes the entry for `key` from the cache, returning its [`Handle`](PolicyExt::Handle)
    /// if it was present, or `None` if the key was not found.
    #[inline]
    pub fn remove(
        &self,
        py: pyo3::Python<'_>,
        key: &<P::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        let mut lock = self.policy();

        let entry = lock.entry(py, key, &self.shared)?;
        match entry {
            PolicyEntry::Occupied(occupied) => {
                let handle = occupied.remove();
                Ok(Some(handle))
            }
            PolicyEntry::Vacant(_) => Ok(None),
        }
    }

    /// Inserts all key-value pairs from `iterable` into the cache.
    ///
    /// `transform` converts a raw `(key, value)` Python object pair into a
    /// policy-specific [`Handle`](PolicyExt::Handle) before insertion.
    ///
    /// # Supported iterables
    ///
    /// - **`dict`** — detected via a fast [`PyObject_TypeCheck`](pyo3::ffi::PyObject_TypeCheck)
    ///   check and iterated with [`PyDictMethods::items`](pyo3::types::PyDictMethods) to avoid
    ///   the overhead of a generic Python iterator.
    /// - **Any object with an `.items()` method** — covers all cache classes and
    ///   other dict-like types; `.items()` is called and the result is iterated.
    /// - **Any other iterable** — iterated directly, with each element expected to
    ///   unpack as a `(key, value)` pair.
    #[inline]
    pub fn extend<F>(&self, iterable: alias::BoundObject, mut transform: F) -> pyo3::PyResult<()>
    where
        F: FnMut(alias::PyObject, alias::PyObject) -> pyo3::PyResult<P::Handle>,
    {
        use pyo3::types::PyAnyMethods;
        use pyo3::types::PyDictMethods;

        /// How many items are transformed before the lock is taken once for
        /// all of them. This bounds the transient memory of an update, and
        /// an unbounded iterable stays streaming.
        const BATCH_SIZE: usize = 1024;

        let py = iterable.py();

        // The iterable, the extraction and `transform` (with `getsizeof`
        // inside) are Python; they run before the lock is taken.
        let mut batch: VecDeque<P::Handle> = VecDeque::new();

        // Using [pyo3::ffi::PyObject_TypeCheck] and [Bound::cast_unchecked] is so faster than [Bound::cast]
        let is_dictionary = unsafe {
            pyo3::ffi::PyObject_TypeCheck(iterable.as_ptr(), crate::typeref::STD_DICT_TYPE) == 1
        };
        if is_dictionary {
            let dict = unsafe { iterable.cast_unchecked::<pyo3::types::PyDict>() };

            batch.reserve(BATCH_SIZE.min(dict.len()));
            for pair in dict.items() {
                let (key, value) = unsafe {
                    pair.extract::<(alias::PyObject, alias::PyObject)>()
                        .unwrap_unchecked()
                };

                batch.push_back(transform(key, value)?);
                if batch.len() == BATCH_SIZE {
                    self.insert_batch(py, &mut batch)?;
                }
            }
        } else {
            // By this we will support everything has `.items()` attribute,
            // including our cache classes
            let items_iterable = {
                if let Some(items_attribute) = iterable.getattr_opt(c"items")? {
                    items_attribute.call0()?
                } else {
                    iterable
                }
            };

            let hint = unsafe { pyo3::ffi::PyObject_LengthHint(items_iterable.as_ptr(), 0) };
            if hint < 0 {
                return Err(pyo3::PyErr::fetch(py));
            }
            batch.reserve(BATCH_SIZE.min(hint as usize));

            for pair in items_iterable.try_iter()? {
                let pair = pair?;
                let (key, value) = pair.extract::<(alias::PyObject, alias::PyObject)>()?;

                batch.push_back(transform(key, value)?);
                if batch.len() == BATCH_SIZE {
                    self.insert_batch(py, &mut batch)?;
                }
            }
        }

        self.insert_batch(py, &mut batch)
    }

    /// Takes the lock once and inserts every buffered handle. Replaced
    /// handles reuse the space opened at the front of the batch; on an error,
    /// its unprocessed tail stays there too. The buffer is cleared only after
    /// the lock guard is gone, so none of them is destroyed under the lock.
    fn insert_batch(
        &self,
        py: pyo3::Python<'_>,
        batch: &mut VecDeque<P::Handle>,
    ) -> pyo3::PyResult<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let count = batch.len();
        let result = {
            let mut lock = self.policy();
            let mut result = Ok(());

            for _ in 0..count {
                let handle = batch.pop_front().unwrap();

                match insert_inner(&mut lock, &self.shared, py, handle) {
                    // Reuse the batch allocation as the deferred-drop buffer.
                    Ok(Some(old)) => batch.push_back(old),
                    Ok(None) => {}
                    Err(err) => {
                        result = Err(err);
                        break;
                    }
                }
            }

            result
        };

        // The lock is gone: clearing may run Python finalizers.
        batch.clear();
        result
    }

    /// Calls the `evict()` `n` times and returns count of removed items.
    #[inline]
    pub fn drain(
        &self,
        py: pyo3::Python,
        n: pyo3::ffi::Py_ssize_t,
    ) -> pyo3::PyResult<pyo3::ffi::Py_ssize_t> {
        if n <= 0 {
            return Ok(0);
        }

        let mut lock = self.policy();

        let expected = (n as usize).min(lock.len());
        lock.pending_drops().reserve(expected);

        let mut count: pyo3::ffi::Py_ssize_t = 0;
        while count < n {
            match lock.evict(&self.shared) {
                Ok(evicted) => lock.pending_drops().push(evicted),
                Err(err) => {
                    if !err.is_instance_of::<pyo3::exceptions::PyKeyError>(py) {
                        return Err(err);
                    }

                    break;
                }
            }

            count += 1;
        }

        Ok(count)
    }

    #[inline]
    pub fn clone_ref(&self, py: pyo3::Python) -> Self {
        let shared = self.shared.clone_ref(py);
        let policy = self.policy().clone_ref(py);

        Self {
            shared,
            inner: parking_lot::Mutex::new(policy),
        }
    }

    pub fn build_pickle(&self, py: pyo3::Python) -> pyo3::PyResult<pickle::Pickle> {
        let mut builder = pickle::Pickle::builder(py, 4)?;

        let getsizeof: Option<alias::PyObject> = self.shared.getsizeof().clone_ref(py).into();

        builder
            .push(self.shared.maxsize())?
            .push(getsizeof)?
            .push(self.shared.global_ttl())?;

        let mut tuple = builder.begin_tuple(P::PICKLE_SIZE)?;
        self.policy().build_pickle(&mut tuple)?;
        tuple.end()?;

        Ok(builder.finish())
    }
}

impl<P: PolicyExt> Wrapped<P> {
    pub fn from_pickle(py: pyo3::Python<'_>, state: alias::PyObject) -> pyo3::PyResult<Self> {
        let tuple = state.into_bound(py).cast_into::<pyo3::types::PyTuple>()?;

        let maxsize: usize = tuple.get_item(0)?.extract()?;
        let getsizeof: Option<alias::PyObject> = tuple.get_item(1)?.extract()?;
        let global_ttl: Option<f64> = tuple.get_item(2)?.extract()?;

        if global_ttl.is_some_and(|x| x < 0.0) {
            return Err(new_py_error!(PyValueError, "global_ttl is negative"));
        }

        let builded = tuple.get_item(3)?.cast_into::<pyo3::types::PyTuple>()?;

        let (shared, inner) = P::from_pickle(
            maxsize,
            getsizeof,
            global_ttl.map(std::time::Duration::from_secs_f64),
            builded,
        )?;

        Ok(Self {
            shared,
            inner: parking_lot::Mutex::new(inner),
        })
    }
}
