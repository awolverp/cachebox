//! The TTL Policy

use crate::hashedkey::HashedKey;
use hashbrown::raw::RawTable;
use std::{collections::VecDeque, time};

pub struct TTLElement {
    pub key: HashedKey,
    pub value: pyo3::PyObject,
    pub expire: time::SystemTime,
}

/// see [`FIFOPolicy`](struct@crate::internal::FIFOPolicy) to find out fields
pub struct TTLPolicy {
    pub table: RawTable<usize>,
    pub entries: VecDeque<TTLElement>,
    pub maxsize: core::num::NonZeroUsize,
    pub ttl: time::Duration,
    pub n_shifts: usize,
}

impl TTLPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize, ttl: f64) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            entries: VecDeque::new(),
            maxsize,
            n_shifts: 0,
            ttl: time::Duration::from_secs_f64(ttl),
        })
    }

    #[inline]
    fn decrement_indexes(&mut self, start: usize, end: usize) {
        if start <= 1 && end == self.entries.len() && self.n_shifts < super::MAX_N_SHIFT {
            self.n_shifts += 1;
            return;
        }

        if (end - start) > self.table.buckets() / 2 {
            unsafe {
                for bucket in self.table.iter() {
                    let i = bucket.as_mut();
                    if start <= (*i) - self.n_shifts && (*i) - self.n_shifts < end {
                        *i -= 1;
                    }
                }
            }
        } else {
            let shifted = self.entries.range(start..end);
            for (i, entry) in (start..end).zip(shifted) {
                #[cfg(debug_assertions)]
                let old = self
                    .table
                    .get_mut(entry.key.hash, |x| (*x) - self.n_shifts == i)
                    .expect("index not found");

                #[cfg(not(debug_assertions))]
                let old = unsafe {
                    self.table
                        .get_mut(entry.key.hash, |x| (*x) - self.n_shifts == i)
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
    unsafe fn insert_unchecked(&mut self, element: TTLElement) -> Option<pyo3::PyObject> {
        match self.table.find_or_find_insert_slot(
            element.key.hash,
            |index| element.key == self.entries[(*index) - self.n_shifts].key,
            |index| self.entries[(*index) - self.n_shifts].key.hash,
        ) {
            Ok(bucket) => {
                let index = unsafe { bucket.as_ref() };
                let m = &mut self.entries[(*index) - self.n_shifts];

                m.expire = element.expire;
                Some(core::mem::replace(&mut m.value, element.value))
            }
            Err(slot) => {
                unsafe {
                    self.table.insert_in_slot(
                        element.key.hash,
                        slot,
                        self.entries.len() + self.n_shifts,
                    );
                }
                self.entries.push_back(element);
                None
            }
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: HashedKey,
        value: pyo3::PyObject,
        expire: bool,
    ) -> Option<pyo3::PyObject> {
        if expire {
            self.expire();
        }

        if self.table.len() >= self.maxsize.get() && !self.contains_key(&key) {
            self.popitem().unwrap();
        }

        unsafe {
            self.insert_unchecked(TTLElement {
                key,
                value,
                expire: time::SystemTime::now() + self.ttl,
            })
        }
    }

    #[inline]
    pub fn expire(&mut self) {
        while !self.entries.is_empty() {
            if self.entries[0].expire > time::SystemTime::now() {
                break;
            }

            unsafe {
                self.popitem().unwrap_unchecked();
            }
        }
    }

    #[inline]
    pub fn popitem(&mut self) -> Option<TTLElement> {
        let ret = self.entries.pop_front()?;

        #[cfg(debug_assertions)]
        self.table
            .remove_entry(ret.key.hash, |index| (*index) - self.n_shifts == 0)
            .expect("popitem key not found.");

        #[cfg(not(debug_assertions))]
        unsafe {
            self.table
                .remove_entry(ret.key.hash, |index| (*index) - self.n_shifts == 0)
                .unwrap_unchecked();
        }

        self.decrement_indexes(1, self.entries.len());
        Some(ret)
    }

    #[inline]
    pub fn contains_key(&self, key: &HashedKey) -> bool {
        match self
            .table
            .find(key.hash, |x| &self.entries[(*x) - self.n_shifts].key == key)
            .map(|x| unsafe { x.as_ref() })
        {
            Some(index) => self.entries[(*index) - self.n_shifts].expire > time::SystemTime::now(),
            None => false,
        }
    }

    #[inline]
    pub fn get(&self, key: &HashedKey) -> Option<&TTLElement> {
        match self
            .table
            .find(key.hash, |x| &self.entries[(*x) - self.n_shifts].key == key)
            .map(|bucket| unsafe { bucket.as_ref() })
        {
            Some(index) => {
                let m = &self.entries[(*index) - self.n_shifts];
                if m.expire > time::SystemTime::now() {
                    Some(m)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashedKey) -> Option<TTLElement> {
        match self
            .table
            .remove_entry(key.hash, |x| key == &self.entries[(*x) - self.n_shifts].key)
            .map(|x| x - self.n_shifts)
        {
            Some(index) => {
                self.decrement_indexes(index + 1, self.entries.len());
                let m = self.entries.remove(index).unwrap();

                if m.expire > time::SystemTime::now() {
                    Some(m)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn update(&mut self, py: pyo3::Python<'_>, iterable: pyo3::PyObject) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        self.expire();

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe {
                iterable
                    .downcast_bound::<pyo3::types::PyDict>(py)
                    .unwrap_unchecked()
            };

            for (key, value) in dict.iter() {
                let hk = unsafe { HashedKey::from_pyobject(py, key.unbind()).unwrap_unchecked() };
                self.insert(hk, value.unbind(), false);
            }

            Ok(())
        } else {
            for pair in iterable.bind(py).iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = HashedKey::from_pyobject(py, key)?;
                self.insert(hk, value, false);
            }

            Ok(())
        }
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> TTLIterator {
        let (a, b) = self.entries.as_slices();

        TTLIterator {
            first: crate::util::NoLifetimeSliceIter {
                slice: a.as_ptr(),
                index: 0,
                len: a.len(),
            },
            second: crate::util::NoLifetimeSliceIter {
                slice: b.as_ptr(),
                index: 0,
                len: b.len(),
            },
        }
    }

    #[inline(always)]
    pub fn shrink_to_fit(&mut self) {
        self.expire();

        self.entries.shrink_to_fit();
        self.table
            .shrink_to(0, |x| self.entries[(*x) - self.n_shifts].key.hash)
    }

    #[inline]
    pub unsafe fn to_pickle(
        &self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let list = pyo3::ffi::PyList_New(0);
        if list.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        for element in self.entries.iter() {
            let tp = tuple!(
                py,
                3,
                0 => element.key.key.clone_ref(py).as_ptr(),
                1 => element.value.clone_ref(py).as_ptr(),
                2 => pyo3::ffi::PyFloat_FromDouble(
                    element.expire.duration_since(time::UNIX_EPOCH).unwrap_unchecked().as_secs_f64()
                ),
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
        let ttl = pyo3::ffi::PyFloat_FromDouble(self.ttl.as_secs_f64());

        tuple!(
            py,
            4,
            0 => maxsize,
            1 => list,
            2 => capacity,
            3 => ttl,
        )
    }

    #[allow(clippy::wrong_self_convention)]
    #[inline]
    pub unsafe fn from_pickle(
        &mut self,
        py: pyo3::Python<'_>,
        state: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::PyAnyMethods;

        tuple!(check state, size=4)?;
        let (maxsize, iterable, capacity) = extract_pickle_tuple!(py, state);

        // SAFETY: we check `iterable` type in `extract_pickle_tuple` macro
        if maxsize < (pyo3::ffi::PyObject_Size(iterable.as_ptr()) as usize) {
            return Err(err!(
                pyo3::exceptions::PyValueError,
                "the iterable object size is more than maxsize!"
            ));
        }

        let ttl = {
            let obj = pyo3::ffi::PyTuple_GetItem(state, 3);
            pyo3::ffi::PyFloat_AsDouble(obj)
        };

        let mut new = Self::new(maxsize, capacity, ttl)?;

        for pair in iterable.bind(py).iter()? {
            let (key, value, timestamp) =
                pair?.extract::<(pyo3::PyObject, pyo3::PyObject, f64)>()?;

            let hk = HashedKey::from_pyobject(py, key)?;

            // SAFETY: we don't need to check maxsize, we sure `len(iterable) <= maxsize`
            new.insert_unchecked(TTLElement {
                key: hk,
                value,
                expire: time::UNIX_EPOCH + time::Duration::from_secs_f64(timestamp),
            });
        }

        new.shrink_to_fit();

        *self = new;
        Ok(())
    }
}

impl PartialEq for TTLPolicy {
    fn eq(&self, other: &Self) -> bool {
        if self.maxsize != other.maxsize || self.ttl != other.ttl {
            return false;
        }

        if self.entries.len() != other.entries.len() {
            return false;
        }

        for index in 0..self.entries.len() {
            let element1 = &self.entries[index];
            let element2 = &other.entries[index];

            if element1.key.hash != element2.key.hash
                || !pyobject_eq!(element1.key.key, element2.key.key)
                || !pyobject_eq!(element1.value, element2.value)
            {
                return false;
            }
        }

        true
    }
}

impl Eq for TTLPolicy {}

pub struct TTLIterator {
    pub first: crate::util::NoLifetimeSliceIter<TTLElement>,
    pub second: crate::util::NoLifetimeSliceIter<TTLElement>,
}

impl Iterator for TTLIterator {
    type Item = *const TTLElement;

    fn next(&mut self) -> Option<Self::Item> {
        match self.first.next() {
            Some(val) => Some(val),
            None => {
                core::mem::swap(&mut self.first, &mut self.second);
                self.first.next()
            }
        }
    }
}

unsafe impl Send for TTLIterator {}
unsafe impl Sync for TTLIterator {}
