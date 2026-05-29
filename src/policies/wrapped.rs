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
    pub fn policy(&self) -> parking_lot::MutexGuard<'_, P> {
        self.inner.lock()
    }
}

#[inline(always)]
fn insert_inner<P: PolicyExt>(
    lock: &mut parking_lot::MutexGuard<'_, P>,
    shared: &P::Shared,
    py: pyo3::Python<'_>,
    handle: P::Handle,
) -> pyo3::PyResult<Option<P::Handle>> {
    let handle_size = handle.size();

    if handle_size > shared.maxsize() {
        return Err(new_py_error!(
            PyOverflowError,
            "handle size is more than the configured maximum size"
        ));
    }

    let result = match lock.entry(py, handle.key(), shared)? {
        PolicyEntry::Occupied(occupied) => Some(occupied.replace(handle)),
        PolicyEntry::Vacant(mut vacant) => {
            // Evict if need
            while vacant.would_exceed(handle_size) {
                vacant.evict()?;
            }

            vacant.insert(handle);
            None
        }
    };

    if result.is_some() {
        // For the `PolicyEntry::Occupied` case, evict after replacement
        while lock.current_size() > shared.maxsize() {
            lock.evict(shared)?;
        }
    }

    Ok(result)
}

// Duplicate methods across all policies
impl<P: PolicyExt> Wrapped<P> {
    /// Returns the remaining size. Equals to `maxsize - current_size`.
    #[inline]
    pub fn remaining_size(&self) -> usize {
        let policy = self.inner.lock();
        self.shared.maxsize().saturating_sub(policy.current_size())
    }

    /// Returns `true` if the cache contains an entry for `key`.
    #[inline]
    pub fn contains(
        &self,
        py: pyo3::Python<'_>,
        key: &<P::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<bool> {
        let mut lock = self.inner.lock();

        let handle = lock.get(py, key)?;
        Ok(handle.is_some())
    }

    /// Inserts a [`Handle`](PolicyExt::Handle) into the cache, evicting entries as needed
    /// to stay within the size budget before inserting.
    ///
    /// - If the key was already present, the old handle is replaced and returned as `Some`.
    /// - If the key was absent, the handle is inserted and `None` is returned.
    #[inline]
    pub fn insert(
        &self,
        py: pyo3::Python<'_>,
        handle: P::Handle,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        let mut lock = self.inner.lock();
        insert_inner(&mut lock, &self.shared, py, handle)
    }

    /// Removes the entry for `key` from the cache, returning its [`Handle`](PolicyExt::Handle)
    /// if it was present, or `None` if the key was not found.
    #[inline]
    pub fn remove(
        &self,
        py: pyo3::Python<'_>,
        key: &<P::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        let mut lock = self.inner.lock();

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

        let mut lock = self.inner.lock();

        // Using [pyo3::ffi::PyObject_TypeCheck] and [Bound::cast_unchecked] is so faster than [Bound::cast]
        let is_dictionary = unsafe {
            pyo3::ffi::PyObject_TypeCheck(iterable.as_ptr(), crate::typeref::STD_DICT_TYPE) == 1
        };
        if is_dictionary {
            let dict = unsafe { iterable.cast_unchecked::<pyo3::types::PyDict>() };

            for pair in dict.items() {
                let (key, value) = unsafe {
                    pair.extract::<(alias::PyObject, alias::PyObject)>()
                        .unwrap_unchecked()
                };

                insert_inner(&mut lock, &self.shared, pair.py(), transform(key, value)?)?;
            }

            return Ok(());
        }

        // By this we will support everything has `.items()` attribute,
        // including our cache classes
        let items_iterable = {
            if let Some(items_attribute) = iterable.getattr_opt(c"items")? {
                items_attribute.call0()?
            } else {
                iterable
            }
        };

        for pair in items_iterable.try_iter()? {
            let pair = pair?;
            let (key, value) = pair.extract::<(alias::PyObject, alias::PyObject)>()?;

            insert_inner(&mut lock, &self.shared, pair.py(), transform(key, value)?)?;
        }

        Ok(())
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

        let mut lock = self.inner.lock();

        let mut count: pyo3::ffi::Py_ssize_t = 0;
        while count < n {
            match lock.evict(&self.shared) {
                Ok(_) => {}
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
        let policy = self.inner.lock().clone_ref(py);

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
        self.inner.lock().build_pickle(&mut tuple)?;
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
            global_ttl.map(|x| std::time::Duration::from_secs_f64(x)),
            builded,
        )?;

        Ok(Self {
            shared,
            inner: parking_lot::Mutex::new(inner),
        })
    }
}
