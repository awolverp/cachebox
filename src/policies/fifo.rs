//! The FIFO policy, This is inspired by Rust's indexmap with some changes.

use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TryFindMethods;

use std::collections::VecDeque;

const MAX_N_SHIFT: usize = usize::MAX - (isize::MAX as usize);

pub struct FIFOPolicy {
    /// We set [Vec] objects indexes in hashtable to make search O(1). hashtable is unordered,
    /// that is why we are using [Vec].
    table: hashbrown::raw::RawTable<usize>,

    /// Keep objects in order.
    entries: VecDeque<(PreHashObject, pyo3::PyObject)>,
    maxsize: core::num::NonZeroUsize,

    /// When we pop front an object from entries, two operations have to do:
    /// 1. Shift all elements in vector.
    /// 2. Decrement all indexes in hashtable.
    ///
    /// these are expensive operations in large elements;
    /// - We removed first operation by using [`std::collections::VecDeque`] instead of [`Vec`]
    /// - We removed second operation by using this variable: Instead of decrement indexes in hashtable,
    ///   we will increment this variable.
    n_shifts: usize,

    pub observed: Observed,
}

pub struct FIFOPolicyOccupied<'a> {
    instance: &'a mut FIFOPolicy,
    bucket: hashbrown::raw::Bucket<usize>,
}

pub struct FIFOPolicyAbsent<'a> {
    instance: &'a mut FIFOPolicy,
}

impl FIFOPolicy {
    #[inline]
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            entries: VecDeque::new(),
            maxsize,
            n_shifts: 0,
            observed: Observed::new(),
        })
    }

    #[inline]
    pub fn maxsize(&self) -> usize {
        self.maxsize.get()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.table.len() == self.maxsize.get()
    }

    #[inline]
    pub fn capacity(&self) -> (usize, usize) {
        (self.table.capacity(), self.entries.capacity())
    }

    // #[inline]
    // pub fn iter(&self) -> hashbrown::raw::RawIter<(PreHashObject, pyo3::PyObject)> {
    //     unsafe { self.table.iter() }
    // }

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
                    .get_mut(entry.0.hash, |x| (*x) - self.n_shifts == i)
                    .expect("index not found");

                *old -= 1;
            }
        }
    }

    #[inline]
    pub fn popitem(
        &mut self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<Option<(PreHashObject, pyo3::PyObject)>> {
        let ret = self.entries.front();
        if ret.is_none() {
            return Ok(None);
        }

        let ret = unsafe { ret.unwrap_unchecked() };

        match self.table.try_find(ret.0.hash, |x| {
            self.entries[(*x) - self.n_shifts].0.equal(py, &ret.0)
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

    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<FIFOPolicyOccupied, FIFOPolicyAbsent>> {
        match self
            .table
            .try_find(key.hash, |x| self.entries[(*x) - self.n_shifts].0.equal(py, key))?
        {
            Some(bucket) => {
                Ok(
                    Entry::Occupied(FIFOPolicyOccupied { instance: self, bucket })
                )
            }
            None => {
                Ok(
                    Entry::Absent(FIFOPolicyAbsent { instance: self })
                )
            },
        }
    }

    pub fn lookup(
        &self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&pyo3::PyObject>> {
        match self
            .table
            .try_find(key.hash, |x| {
                self.entries[(*x) - self.n_shifts].0.equal(py, key)
            })?
            .map(|bucket| unsafe { bucket.as_ref() })
        {
            Some(index) => Ok(Some(&self.entries[(*index) - self.n_shifts].1)),
            None => Ok(None),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.table.clear();
        self.entries.clear();
        self.n_shifts = 0;
        self.observed.change();
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.table.shrink_to(self.table.len(), |x| {
            self.entries[(*x) - self.n_shifts].0.hash
        });
        self.entries.shrink_to_fit();
        self.observed.change();
    }

    pub fn entries_iter(
        &self,
    ) -> std::collections::vec_deque::Iter<'_, (PreHashObject, pyo3::PyObject)> {
        self.entries.iter()
    }
}

impl<'a> FIFOPolicyOccupied<'a> {
    #[inline]
    pub fn update(&mut self, value: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        let index = unsafe { self.bucket.as_ref() };
        let item = &mut self.instance.entries[index - self.instance.n_shifts];
        let old_value = std::mem::replace(&mut item.1, value);
        self.instance.observed.change();
        Ok(old_value)
    }

    #[inline]
    pub fn remove(self) -> (PreHashObject, pyo3::PyObject) {
        // let (PreHashObject { hash, .. }, _) = &self.instance.entries[self.index - self.instance.n_shifts];
        let (mut index, _) = unsafe { self.instance.table.remove(self.bucket) };
        index = index - self.instance.n_shifts;

        self.instance
            .decrement_indexes(index + 1, self.instance.entries.len());

        let m = self.instance.entries.remove(index).unwrap();

        self.instance.observed.change();
        m
    }

    #[inline]
    pub fn into_value(self) -> &'a mut (PreHashObject, pyo3::PyObject) {
        let index = unsafe { self.bucket.as_ref() };
        &mut self.instance.entries[index - self.instance.n_shifts]
    }
}

impl<'a> FIFOPolicyAbsent<'a> {
    #[inline]
    pub fn insert(
        self,
        py: pyo3::Python<'_>,
        key: PreHashObject,
        value: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        if self.instance.table.len() >= self.instance.maxsize.get() {
            self.instance.popitem(py)?;
        }

        self.instance.table.insert(
            key.hash,
            self.instance.entries.len() + self.instance.n_shifts,
            |index| {
                self.instance.entries[(*index) - self.instance.n_shifts]
                    .0
                    .hash
            },
        );
        self.instance.entries.push_back((key, value));

        self.instance.observed.change();
        Ok(())
    }
}
