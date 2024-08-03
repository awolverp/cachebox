//! The FIFO cache, This is inspired by Rust's indexmap with some changes.

use crate::hashedkey::HashedKey;
use hashbrown::raw::RawTable;

const MAX_N_SHIFT: usize = usize::MAX - (isize::MAX as usize);

pub struct FIFOPolicy {
    /// We set [Vec] objects indexes in hashtable to make search O(1). hashtable is unordered,
    /// that is why we are using [Vec].
    pub table: RawTable<usize>,

    /// Keep objects in order.
    pub entries: Vec<(HashedKey, pyo3::PyObject)>,
    pub maxsize: core::num::NonZeroUsize,

    /// When we pop front an object from entries, two operations have to do:
    /// 1. Shift all elements in vector.
    /// 2. Decrement all indexes in hashtable.
    ///
    /// these are expensive operations in large elements; We removed second operation by
    /// using this variable: Instead of decrement indexes in hashtable, we will
    /// increment this variable.
    pub n_shifts: usize,
}

impl FIFOPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            entries: Vec::new(),
            maxsize,
            n_shifts: 0,
        })
    }

    fn decrement_indexes(&mut self, start: usize, end: usize) {
        if start <= 1 && end == self.entries.len() && self.n_shifts < MAX_N_SHIFT {
            self.n_shifts += 1;
            return;
        }

        let shifted = &self.entries[start..end];
        if shifted.len() > self.table.buckets() / 2 {
            unsafe {
                for bucket in self.table.iter() {
                    let i = bucket.as_mut();
                    if start <= (*i) - self.n_shifts && (*i) - self.n_shifts < end {
                        *i -= 1;
                    }
                }
            }
        } else {
            for (i, entry) in (start..end).zip(shifted) {
                #[cfg(debug_assertions)]
                let old = self
                    .table
                    .get_mut(entry.0.hash, |x| (*x) - self.n_shifts == i)
                    .expect("index not found");

                #[cfg(not(debug_assertions))]
                let old = unsafe {
                    self.table
                        .get_mut(entry.0.hash, |x| (*x) - self.n_shifts == i)
                        .unwrap_unchecked()
                };

                *old -= 1;
            }
        }
    }

    /// # Safety
    ///
    /// This method is unsafe because does not checks the maxsize and this
    /// may occurred errors and bad situations in future if you don't care about
    /// maxsize.
    #[inline]
    unsafe fn insert_unchecked(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
    ) -> Option<pyo3::PyObject> {
        match self.table.find_or_find_insert_slot(
            key.hash,
            |index| key == self.entries[(*index) - self.n_shifts].0,
            |index| self.entries[(*index) - self.n_shifts].0.hash,
        ) {
            Ok(bucket) => {
                let index = unsafe { bucket.as_ref() };
                Some(core::mem::replace(
                    &mut self.entries[(*index) - self.n_shifts].1,
                    value,
                ))
            }
            Err(slot) => {
                unsafe {
                    self.table
                        .insert_in_slot(key.hash, slot, self.entries.len() + self.n_shifts);
                }
                self.entries.push((key, value));
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
            && self
                .table
                .find(key.hash, |index| {
                    key == self.entries[(*index) - self.n_shifts].0
                })
                .is_none()
        {
            #[cfg(debug_assertions)]
            self.popitem().unwrap();

            #[cfg(not(debug_assertions))]
            unsafe {
                self.popitem().unwrap_unchecked();
            }
        }

        Ok(unsafe { self.insert_unchecked(key, value) })
    }

    #[inline]
    pub fn popitem(&mut self) -> Option<(HashedKey, pyo3::PyObject)> {
        if self.entries.is_empty() {
            return None;
        }

        let ret = self.entries.remove(0);

        #[cfg(debug_assertions)]
        self.table
            .remove_entry(ret.0.hash, |index| (*index) - self.n_shifts == 0)
            .expect("popitem key not found.");

        #[cfg(not(debug_assertions))]
        unsafe {
            self.table
                .remove_entry(ret.0.hash, |index| (*index) - self.n_shifts == 0)
                .unwrap_unchecked();
        }

        self.decrement_indexes(1, self.entries.len());
        Some(ret)
    }

    #[inline]
    pub fn get(&self, key: &HashedKey) -> Option<&pyo3::PyObject> {
        match self
            .table
            .find(key.hash, |x| &self.entries[(*x) - self.n_shifts].0 == key)
            .map(|bucket| unsafe { bucket.as_ref() })
        {
            Some(index) => Some(&self.entries[(*index) - self.n_shifts].1),
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<(HashedKey, pyo3::PyObject)> {
        match self
            .table
            .remove_entry(key.hash, |x| key == &self.entries[(*x) - self.n_shifts].0)
            .map(|x| x - self.n_shifts)
        {
            Some(index) => {
                self.decrement_indexes(index + 1, self.entries.len());
                Some(self.entries.remove(index))
            }
            None => None,
        }
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        self.table
            .find(key.hash, |x| &self.entries[(*x) - self.n_shifts].0 == key)
            .is_some()
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
            for pair in iterable.bind(py).iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = HashedKey::from_pyobject(py, key)?;
                self.insert(hk, value)?;
            }

            Ok(())
        }
    }

    pub fn as_ptr(&self) -> FIFOVecPtr {
        FIFOVecPtr {
            entries: self.entries.as_ptr(),
            offset: 0,
            length: self.entries.len(),
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.entries.shrink_to_fit();
        self.table
            .shrink_to(0, |x| self.entries[(*x) - self.n_shifts].0.hash)
    }

    pub unsafe fn to_pickle(
        &self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let list = pyo3::ffi::PyList_New(0);
        if list.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for (hk, val) in self.entries.iter() {
            let tp = tuple!(
                py,
                2,
                0 => hk.key.clone_ref(py).as_ptr(),
                1 => val.clone_ref(py).as_ptr(),
            );

            if let Err(x) = tp {
                pyo3::ffi::Py_DECREF(list);
                return Err(x);
            }

            if pyo3::ffi::PyList_Append(list, tp.unwrap_unchecked()) == -1 {
                pyo3::ffi::Py_DECREF(list);
                return Err(pyo3::PyErr::fetch(py));
            }
        }

        let maxsize = pyo3::ffi::PyLong_FromSize_t(self.maxsize.get());
        let capacity = pyo3::ffi::PyLong_FromSize_t(self.table.capacity());

        tuple!(
            py,
            3,
            0 => maxsize,
            1 => list,
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

impl PartialEq for FIFOPolicy {
    fn eq(&self, other: &Self) -> bool {
        if self.maxsize != other.maxsize {
            return false;
        }

        if self.entries.len() != other.entries.len() {
            return false;
        }

        for index in 0..self.entries.len() {
            let (key1, val1) = &self.entries[index];
            let (key2, val2) = &other.entries[index];

            if key1.hash != key2.hash
                || !pyobject_eq!(key1.key, key2.key)
                || !pyobject_eq!(val1, val2)
            {
                return false;
            }
        }

        true
    }
}

impl Eq for FIFOPolicy {}

pub struct FIFOVecPtr {
    pub entries: *const (HashedKey, pyo3::PyObject),
    pub offset: usize,
    pub length: usize,
}

unsafe impl Send for FIFOVecPtr {}
unsafe impl Sync for FIFOVecPtr {}
