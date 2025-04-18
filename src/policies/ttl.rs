use super::fifo::MAX_N_SHIFT;
use crate::common::AbsentSituation;
use crate::common::Entry;
use crate::common::NoLifetimeSliceIter;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TimeToLivePair;
use crate::common::TryFindMethods;

use std::collections::VecDeque;

pub struct TTLPolicy {
    // See FIFOPolicy to find out fields
    table: hashbrown::raw::RawTable<usize>,
    entries: VecDeque<TimeToLivePair>,
    maxsize: core::num::NonZeroUsize,
    ttl: std::time::Duration,
    n_shifts: usize,
    pub observed: Observed,
}

pub struct TTLPolicyOccupied<'a> {
    instance: &'a mut TTLPolicy,
    bucket: hashbrown::raw::Bucket<usize>,
}

pub struct TTLPolicyAbsent<'a> {
    instance: &'a mut TTLPolicy,
    situation: AbsentSituation<usize>,
}

pub struct TTLIterator {
    first: NoLifetimeSliceIter<TimeToLivePair>,
    second: NoLifetimeSliceIter<TimeToLivePair>,
}

impl TTLPolicy {
    pub fn new(maxsize: usize, mut capacity: usize, secs: f64) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            entries: VecDeque::new(),
            maxsize,
            ttl: std::time::Duration::from_secs_f64(secs),
            n_shifts: 0,
            observed: Observed::new(),
        })
    }

    pub fn maxsize(&self) -> usize {
        self.maxsize.get()
    }

    pub fn ttl(&self) -> std::time::Duration {
        self.ttl
    }

    #[inline]
    pub fn real_len(&self) -> usize {
        let now = std::time::SystemTime::now();
        let mut c = 0usize;

        for item in &self.entries {
            if !item.is_expired(now) {
                break;
            }

            c += 1;
        }

        self.table.len() - c
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.real_len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.real_len() == self.maxsize.get()
    }

    pub fn capacity(&self) -> (usize, usize) {
        (self.table.capacity(), self.entries.capacity())
    }

    #[inline]
    fn decrement_indexes(&mut self, start: usize, end: usize) {
        if start <= 1 && end == self.entries.len() && self.n_shifts < MAX_N_SHIFT {
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
                let old = self
                    .table
                    .get_mut(entry.key.hash, |x| (*x) - self.n_shifts == i)
                    .expect("index not found");

                *old -= 1;
            }
        }
    }

    #[inline]
    pub fn expire(&mut self, py: pyo3::Python<'_>) {
        let now = std::time::SystemTime::now();

        while let Some(e) = self.entries.front() {
            if !e.is_expired(now) {
                break;
            }

            unsafe {
                self.popitem(py).unwrap_unchecked();
            }
        }
    }

    #[inline]
    pub fn popitem(&mut self, py: pyo3::Python<'_>) -> pyo3::PyResult<Option<TimeToLivePair>> {
        let ret = self.entries.front();
        if ret.is_none() {
            return Ok(None);
        }

        let ret = unsafe { ret.unwrap_unchecked() };

        match self.table.try_find(ret.key.hash, |x| {
            self.entries[(*x) - self.n_shifts].key.equal(py, &ret.key)
        })? {
            Some(bucket) => {
                unsafe { self.table.remove(bucket) };
            }
            None => unreachable!("popitem key not found in table"),
        }

        let ret = unsafe { self.entries.pop_front().unwrap_unchecked() };

        self.observed.change();

        self.decrement_indexes(1, self.entries.len());
        Ok(Some(ret))
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<TTLPolicyOccupied, TTLPolicyAbsent>> {
        match self
            .table
            .try_find(key.hash, |x| self.entries[(*x) - self.n_shifts].key.equal(py, key))?
        {
            Some(bucket) => {
                let pair = &self.entries[unsafe { *bucket.as_ptr() } - self.n_shifts];

                if !pair.is_expired(std::time::SystemTime::now()) {
                    Ok(Entry::Occupied(TTLPolicyOccupied { instance: self, bucket }))
                } else {
                    Ok(Entry::Absent(TTLPolicyAbsent { instance: self, situation: AbsentSituation::Expired(bucket) }))
                }
            }
            None => {
                Ok(
                    Entry::Absent(TTLPolicyAbsent { instance: self, situation: AbsentSituation::None })
                )
            },
        }
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry_with_slot(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<TTLPolicyOccupied, TTLPolicyAbsent>> {
        match self.table.try_find_or_find_insert_slot(
            key.hash,
            |x| self.entries[(*x) - self.n_shifts].key.equal(py, key),
            |x| self.entries[(*x) - self.n_shifts].key.hash,
        )? {
            Ok(bucket) => {
                let pair = &self.entries[unsafe { *bucket.as_ptr() } - self.n_shifts];

                if !pair.is_expired(std::time::SystemTime::now()) {
                    Ok(Entry::Occupied(TTLPolicyOccupied { instance: self, bucket }))
                } else {
                    Ok(Entry::Absent(TTLPolicyAbsent { instance: self, situation: AbsentSituation::Expired(bucket) }))
                }
            },
            Err(insert_slot) => {
                Ok(
                    Entry::Absent(TTLPolicyAbsent { instance: self, situation: AbsentSituation::Slot(insert_slot) })
                )
            },
        }
    }

    #[inline]
    pub fn lookup(
        &self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&TimeToLivePair>> {
        match self
            .table
            .try_find(key.hash, |x| {
                self.entries[(*x) - self.n_shifts].key.equal(py, key)
            })?
            .map(|bucket| unsafe { bucket.as_ref() })
        {
            Some(index) => {
                let pair = &self.entries[(*index) - self.n_shifts];

                if !pair.is_expired(std::time::SystemTime::now()) {
                    Ok(Some(pair))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.entries.clear();
        self.n_shifts = 0;
        self.observed.change();
    }

    pub fn shrink_to_fit(&mut self, py: pyo3::Python<'_>) {
        self.expire(py);

        self.table.shrink_to(self.table.len(), |x| {
            self.entries[(*x) - self.n_shifts].key.hash
        });
        self.entries.shrink_to_fit();
        self.observed.change();
    }

    #[inline]
    pub fn extend(&mut self, py: pyo3::Python<'_>, iterable: pyo3::PyObject) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe {
                iterable
                    .downcast_bound::<pyo3::types::PyDict>(py)
                    .unwrap_unchecked()
            };

            for (key, value) in dict.iter() {
                let hk =
                    unsafe { PreHashObject::from_pyobject(py, key.unbind()).unwrap_unchecked() };

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(value.unbind())?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(py, hk, value.unbind())?;
                    }
                }
            }
        } else {
            for pair in iterable.bind(py).try_iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(value)?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(py, hk, value)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn entries_iter(&self) -> std::collections::vec_deque::Iter<'_, TimeToLivePair> {
        self.entries.iter()
    }

    pub fn equal(&self, py: pyo3::Python<'_>, other: &Self) -> pyo3::PyResult<bool> {
        if self.maxsize != other.maxsize {
            return Ok(false);
        }

        if self.real_len() != other.real_len() {
            return Ok(false);
        }

        let now = std::time::SystemTime::now();

        unsafe {
            for index1 in self.table.iter().map(|x| x.as_ref()) {
                let pair1 = &self.entries[(*index1) - self.n_shifts];

                if pair1.is_expired(now) {
                    continue;
                }

                match other.table.try_find(pair1.key.hash, |x| {
                    pair1
                        .key
                        .equal(py, &other.entries[(*x) - other.n_shifts].key)
                })? {
                    Some(bucket) => {
                        let pair2 = &other.entries[(*bucket.as_ref()) - other.n_shifts];

                        if pair2.is_expired(now) {
                            return Ok(false);
                        }

                        if !crate::common::pyobject_equal(
                            py,
                            pair1.value.as_ptr(),
                            pair2.value.as_ptr(),
                        )? {
                            return Ok(false);
                        }
                    }
                    None => return Ok(false),
                }
            }
        }

        Ok(true)
    }

    pub fn iter(&mut self, py: pyo3::Python<'_>) -> TTLIterator {
        self.expire(py);

        let (a, b) = self.entries.as_slices();

        TTLIterator {
            first: NoLifetimeSliceIter::new(a),
            second: NoLifetimeSliceIter::new(b),
        }
    }

    pub fn get_index(&self, n: usize) -> Option<&TimeToLivePair> {
        self.entries.get(n)
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_pickle(
        &mut self,
        py: pyo3::Python<'_>,
        state: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::PyAnyMethods;

        unsafe {
            tuple!(check state, size=4)?;
            let (maxsize, iterable, capacity) = extract_pickle_tuple!(py, state => list);

            // SAFETY: we check `iterable` type in `extract_pickle_tuple` macro
            if maxsize < (pyo3::ffi::PyObject_Size(iterable.as_ptr()) as usize) {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "the iterable object size is more than maxsize!",
                ));
            }

            let ttl = {
                let obj = pyo3::ffi::PyTuple_GetItem(state, 3);
                pyo3::ffi::PyFloat_AsDouble(obj)
            };

            let mut new = Self::new(maxsize, capacity, ttl)?;

            for pair in iterable.bind(py).try_iter()? {
                let (key, value, timestamp) =
                    pair?.extract::<(pyo3::PyObject, pyo3::PyObject, f64)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match new.entry_with_slot(py, &hk)? {
                    Entry::Absent(entry) => {
                        entry.pickle_insert(
                            hk,
                            value,
                            std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(timestamp),
                        )?;
                    }
                    _ => std::hint::unreachable_unchecked(),
                }
            }

            new.expire(py);
            new.shrink_to_fit(py);

            *self = new;
            Ok(())
        }
    }
}

impl<'a> TTLPolicyOccupied<'a> {
    #[inline]
    pub fn update(self, value: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        // We have to move the value to the end of the vector
        let (mut index, slot) = unsafe { self.instance.table.remove(self.bucket.clone()) };
        index -= self.instance.n_shifts;

        self.instance
            .decrement_indexes(index + 1, self.instance.entries.len());

        let mut item = self.instance.entries.remove(index).unwrap();

        item.expire_at = Some(std::time::SystemTime::now() + self.instance.ttl);
        let old_value = std::mem::replace(&mut item.value, value);

        unsafe {
            self.instance.table.insert_in_slot(
                item.key.hash,
                slot,
                self.instance.entries.len() + self.instance.n_shifts,
            );

            self.instance.entries.push_back(item);
        }

        self.instance.observed.change();

        Ok(old_value)
    }

    #[inline]
    pub fn remove(self) -> TimeToLivePair {
        let (mut index, _) = unsafe { self.instance.table.remove(self.bucket) };
        index -= self.instance.n_shifts;

        self.instance
            .decrement_indexes(index + 1, self.instance.entries.len());

        let m = self.instance.entries.remove(index).unwrap();

        self.instance.observed.change();
        m
    }

    pub fn into_value(self) -> &'a mut TimeToLivePair {
        let index = unsafe { self.bucket.as_ref() };
        &mut self.instance.entries[index - self.instance.n_shifts]
    }
}

impl TTLPolicyAbsent<'_> {
    unsafe fn pickle_insert(
        self,
        key: PreHashObject,
        value: pyo3::PyObject,
        expire_at: std::time::SystemTime,
    ) -> pyo3::PyResult<()> {
        match self.situation {
            AbsentSituation::Expired(_) => {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "pikcle object is suspicious!",
                ))
            }
            AbsentSituation::Slot(slot) => unsafe {
                // This means the key is not available and we have insert_slot
                // for inserting it

                // We don't need to check maxsize, we sure `len(iterable) <= maxsize` in loading pickle

                self.instance.table.insert_in_slot(
                    key.hash,
                    slot,
                    self.instance.entries.len() + self.instance.n_shifts,
                );

                self.instance
                    .entries
                    .push_back(TimeToLivePair::new(key, value, Some(expire_at)));
            },
            AbsentSituation::None => unsafe { std::hint::unreachable_unchecked() },
        }

        Ok(())
    }

    #[inline]
    pub fn insert(
        self,
        py: pyo3::Python<'_>,
        key: PreHashObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        let expire_at = std::time::SystemTime::now() + self.instance.ttl;

        match self.situation {
            AbsentSituation::Expired(bucket) => {
                // This means the key is available but expired
                // So we have to move the value to the end of the vector
                // and update the bucket ( like TTLPolicyOccupied::update )
                let (mut index, slot) = unsafe { self.instance.table.remove(bucket) };
                index -= self.instance.n_shifts;

                self.instance
                    .decrement_indexes(index + 1, self.instance.entries.len());

                let mut item = self.instance.entries.remove(index).unwrap();

                item.expire_at = Some(expire_at);
                item.value = value;

                unsafe {
                    self.instance.table.insert_in_slot(
                        item.key.hash,
                        slot,
                        self.instance.entries.len() + self.instance.n_shifts,
                    );

                    self.instance.entries.push_back(item);
                }
            }
            AbsentSituation::Slot(slot) => unsafe {
                // This means the key is not available and we have insert_slot
                // for inserting it

                self.instance.expire(py); // Remove expired pairs to make room for the new pair

                if self.instance.table.len() >= self.instance.maxsize.get() {
                    self.instance.popitem(py)?;
                }

                self.instance.table.insert_in_slot(
                    key.hash,
                    slot,
                    self.instance.entries.len() + self.instance.n_shifts,
                );

                self.instance
                    .entries
                    .push_back(TimeToLivePair::new(key, value, Some(expire_at)));
            },
            AbsentSituation::None => {
                // This is same as AbsentSituation::Slot but we don't have any slot

                self.instance.expire(py); // Remove expired pairs to make room for the new pair

                if self.instance.table.len() >= self.instance.maxsize.get() {
                    self.instance.popitem(py)?;
                }

                self.instance.table.insert(
                    key.hash,
                    self.instance.entries.len() + self.instance.n_shifts,
                    |index| {
                        self.instance.entries[(*index) - self.instance.n_shifts]
                            .key
                            .hash
                    },
                );

                self.instance
                    .entries
                    .push_back(TimeToLivePair::new(key, value, Some(expire_at)));
            }
        }

        self.instance.observed.change();
        Ok(())
    }
}

impl Iterator for TTLIterator {
    type Item = std::ptr::NonNull<TimeToLivePair>;

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
