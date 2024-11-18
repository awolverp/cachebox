//! The bounded cache, away from any algorithms ...

use crate::hashedkey::HashedKey;
use hashbrown::raw::RawTable;

pub struct NoPolicy {
    pub table: RawTable<(HashedKey, pyo3::PyObject)>,
    pub maxsize: core::num::NonZeroUsize,
}

impl NoPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            maxsize,
        })
    }

    /// # Safety
    ///
    /// This method is unsafe because does not checks the maxsize and this
    /// may occurred errors and bad situations in future if you don't care about
    /// maxsize.
    #[inline]
    pub unsafe fn insert_unchecked(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
    ) -> Option<pyo3::PyObject> {
        match self
            .table
            .find_or_find_insert_slot(key.hash, |x| x.0 == key, |x| x.0.hash)
        {
            Ok(bucket) => Some(core::mem::replace(&mut (bucket.as_mut().1), value)),
            Err(slot) => {
                self.table.insert_in_slot(key.hash, slot, (key, value));
                None
            }
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        if self.table.len() >= self.maxsize.get()
            && self.table.find(key.hash, |x| x.0 == key).is_none()
        {
            // There's no algorithm for removing a key-value pair, so we raise PyOverflowError.
            return Err(err!(
                pyo3::exceptions::PyOverflowError,
                "The cache has reached the bound"
            ));
        }

        Ok(unsafe { self.insert_unchecked(key, value) })
    }

    #[inline]
    pub fn get(&self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        self.table
            .find(key.hash, |x| x.0 == *key)
            .map(|bucket| unsafe { &bucket.as_ref().1 })
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<(HashedKey, pyo3::PyObject)> {
        self.table.remove_entry(key.hash, |x| x.0 == *key)
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        self.table.find(key.hash, |x| x.0 == *key).is_some()
    }

    #[inline]
    pub fn update(&mut self, py: pyo3::Python<'_>, iterable: pyo3::PyObject) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe {
                iterable
                    .downcast_bound::<pyo3::types::PyDict>(py)
                    .unwrap_unchecked()
            };

            for (key, value) in dict.iter() {
                let hk = unsafe { HashedKey::from_pyobject(py, key.unbind()).unwrap_unchecked() };
                self.insert(hk, value.unbind())?;
            }

            Ok(())
        } else {
            for pair in iterable.bind(py).try_iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = HashedKey::from_pyobject(py, key)?;
                self.insert(hk, value)?;
            }

            Ok(())
        }
    }

    pub unsafe fn to_pickle(
        &self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let mp = pyo3::ffi::PyDict_New();

        if mp.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for bucket in self.table.iter() {
            let (key, val) = bucket.as_ref();
            // SAFETY: we don't need to check error because we sure about key that is hashable.
            pyo3::ffi::PyDict_SetItem(mp, key.key.as_ptr(), val.as_ptr());
        }

        let maxsize = pyo3::ffi::PyLong_FromSize_t(self.maxsize.get());
        let capacity = pyo3::ffi::PyLong_FromSize_t(self.table.capacity());

        tuple!(
            py,
            3,
            0 => maxsize,
            1 => mp,
            2 => capacity,
        )
    }

    #[allow(clippy::wrong_self_convention)]
    pub unsafe fn from_pickle(
        &mut self,
        py: pyo3::Python<'_>,
        state: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        tuple!(check state, size=3)?;
        let (maxsize, iterable, capacity) = extract_pickle_tuple!(py, state);

        let mut new = Self::new(maxsize, capacity)?;
        new.update(py, iterable)?;

        *self = new;
        Ok(())
    }
}

impl PartialEq for NoPolicy {
    fn eq(&self, other: &Self) -> bool {
        if self.maxsize != other.maxsize {
            return false;
        }

        if self.table.len() != other.table.len() {
            return false;
        }

        #[allow(unused_unsafe)]
        unsafe {
            self.table.iter().all(|bucket| {
                let (key, value) = bucket.as_ref();

                other.get(key).map_or(false, |x| pyobject_eq!(value, x))
            })
        }
    }
}

impl Eq for NoPolicy {}
