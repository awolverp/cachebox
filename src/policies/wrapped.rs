use std::ops::Deref;
use std::ops::DerefMut;

use crate::internal::alias;
use crate::policies::traits::EntryExt;
use crate::policies::traits::HandleExt;
use crate::policies::traits::OccupiedExt;
use crate::policies::traits::PolicyEntry;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::VacantExt;

/// A transparent wrapper over [`PolicyExt`] implementations that adds
/// higher-level methods shared across all policies.
///
/// - [`insert`](Wrapped::insert)
/// - [`remove`](Wrapped::remove)
/// - [`contains`](Wrapped::contains)
/// - [`extend`](Wrapped::extend).
///
/// Because the wrapper is `#[repr(transparent)]` and implements [`Deref`] / [`DerefMut`],
/// all methods of the inner policy `P` are directly accessible without unwrapping.
#[repr(transparent)]
pub struct Wrapped<P: PolicyExt>(P);

impl<P: PolicyExt> Deref for Wrapped<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P: PolicyExt> DerefMut for Wrapped<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P: PolicyExt> Wrapped<P> {
    /// Wraps an existing policy, granting access to the shared higher-level API.
    pub fn new(policy: P) -> Self {
        Self(policy)
    }

    /// Returns the remaining size. Equals to `maxsize - current_size`.
    pub fn remaining_size(&self) -> usize {
        self.maxsize().checked_sub(self.current_size()).unwrap_or(0)
    }

    /// Returns `true` if the cache contains an entry for `key`.
    pub fn contains(
        &mut self,
        py: pyo3::Python<'_>,
        key: &<P::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<bool> {
        let handle = self.0.get(py, key)?;
        Ok(handle.is_some())
    }

    /// Inserts a [`Handle`](PolicyExt::Handle) into the cache, evicting entries as needed
    /// to stay within the size budget before inserting.
    ///
    /// - If the key was already present, the old handle is replaced and returned as `Some`.
    /// - If the key was absent, the handle is inserted and `None` is returned.
    pub fn insert(
        &mut self,
        py: pyo3::Python<'_>,
        handle: P::Handle,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        let entry = self.0.entry(py, handle.key())?;

        match entry {
            PolicyEntry::Occupied(mut occupied) => {
                // Evict if need
                while occupied.would_exceed(handle.size()) {
                    occupied.evict()?;
                }

                Ok(Some(occupied.replace(handle)))
            }
            PolicyEntry::Vacant(mut vacant) => {
                // Evict if need
                while vacant.would_exceed(handle.size()) {
                    vacant.evict()?;
                }

                vacant.insert(handle);
                Ok(None)
            }
        }
    }

    /// Removes the entry for `key` from the cache, returning its [`Handle`](PolicyExt::Handle)
    /// if it was present, or `None` if the key was not found.
    pub fn remove(
        &mut self,
        py: pyo3::Python<'_>,
        key: &<P::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<Option<P::Handle>> {
        let entry = self.0.entry(py, key)?;

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
    pub fn extend<F>(
        &mut self,
        iterable: alias::BoundObject,
        mut transform: F,
    ) -> pyo3::PyResult<()>
    where
        F: FnMut(alias::PyObject, alias::PyObject) -> pyo3::PyResult<P::Handle>,
    {
        use pyo3::types::PyAnyMethods;
        use pyo3::types::PyDictMethods;

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

                self.insert(pair.py(), transform(key, value)?)?;
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

            self.insert(pair.py(), transform(key, value)?)?;
        }

        Ok(())
    }

    /// Calls the `evict()` `n` times and returns count of removed items.
    pub fn drain(
        &mut self,
        py: pyo3::Python,
        n: pyo3::ffi::Py_ssize_t,
    ) -> pyo3::PyResult<pyo3::ffi::Py_ssize_t> {
        if n <= 0 {
            return Ok(0);
        }

        let mut count: pyo3::ffi::Py_ssize_t = 0;
        while count < n {
            match self.0.evict() {
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
}
