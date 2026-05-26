use std::collections::VecDeque;

use crate::hashbrown;
use crate::internal::alias;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;

pub use crate::policies::common::Shared;

macro_rules! get_handle {
    (&$slf:expr, $index:expr) => {
        &$slf.entries[$index - $slf.front_offset]
    };
    (&mut $slf:expr, $index:expr) => {
        &mut $slf.entries[$index - $slf.front_offset]
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExpiresAt {
    SystemTime(std::time::SystemTime),
    Duration(std::time::Duration),
}

impl From<std::time::Duration> for ExpiresAt {
    #[inline]
    fn from(value: std::time::Duration) -> Self {
        Self::Duration(value)
    }
}

impl From<ExpiresAt> for std::time::SystemTime {
    #[inline]
    fn from(value: ExpiresAt) -> Self {
        match value {
            ExpiresAt::Duration(x) => std::time::SystemTime::now() + x,
            ExpiresAt::SystemTime(x) => x,
        }
    }
}

/// A key-value pair with a precomputed hash and combined size.
pub struct ExpiringHandle {
    /// The cache key together with its precomputed hash, avoiding repeated
    /// Python hash calls during table lookups.
    key: utils::PrecomputedHashObject,
    /// The cached value associated with this key.
    value: alias::PyObject,
    /// Size of the key and value as reported by `getsizeof`.
    size: usize,
    /// Configured ttl for handle.
    expires_at: std::time::SystemTime,
}

impl ExpiringHandle {
    /// Creates a new [`Handle`], which calculates the precomputed hash itself.
    #[inline]
    pub fn new(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        expires_at: ExpiresAt,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        Self::with_precomputed_hash_key(
            py,
            getsizeof,
            expires_at,
            utils::PrecomputedHashObject::new(py, key)?,
            value,
        )
    }

    /// Creates a new [`Handle`] from an already-hashed key.
    ///
    /// Prefer this over [`Handle::new`] when the caller has already paid the cost
    /// of computing the hash (e.g. during a table lookup that preceded insertion).
    #[inline]
    pub fn with_precomputed_hash_key(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        expires_at: ExpiresAt,
        key: utils::PrecomputedHashObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        let size = getsizeof.call(py, key.as_ref(), &value)?;
        Ok(Self {
            key,
            value,
            size,
            expires_at: expires_at.into(),
        })
    }

    /// Consumes `self` and returns the [`utils::PrecomputedHashObject`].
    #[inline]
    pub fn into_key(self) -> utils::PrecomputedHashObject {
        self.key
    }

    /// Returns a reference to the value.
    #[inline]
    pub fn value(&self) -> &alias::PyObject {
        &self.value
    }

    /// Consumes `self` and returns the value of the pair.
    #[inline]
    pub fn into_value(self) -> alias::PyObject {
        self.value
    }

    /// Consumes `self` and returns the pair.
    #[inline]
    pub fn into_pair(self) -> (utils::PrecomputedHashObject, alias::PyObject) {
        (self.key, self.value)
    }

    #[inline]
    pub fn expires_at(&self) -> std::time::SystemTime {
        self.expires_at
    }

    #[inline]
    pub fn is_expired(&self, now: std::time::SystemTime) -> bool {
        self.expires_at <= now
    }

    /// Makes a clone of self.
    ///
    /// This creates another pointer to the same object, increasing its reference count.
    #[inline]
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self {
            key: self.key.clone_ref(py),
            value: self.value.clone_ref(py),
            size: self.size,
            expires_at: self.expires_at,
        }
    }
}

impl HandleExt for ExpiringHandle {
    type Key = utils::PrecomputedHashObject;

    #[inline(always)]
    fn key(&self) -> &utils::PrecomputedHashObject {
        &self.key
    }

    #[inline(always)]
    fn size(&self) -> usize {
        self.size
    }
}

/// A view into an occupied entry in [`TTLPolicy`].
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut TTLPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// Raw bucket pointing to the occupied index.
    bucket: hashbrown::raw::Bucket<usize>,
}

impl traits::OccupiedExt for Occupied<'_> {
    type Handle = ExpiringHandle;
    type Shared = Shared;

    #[inline]
    fn replace(self, new: Self::Handle) -> Self::Handle {
        // In update we don't need to increment this; because this does not change the memory address ranges
        // self.shared.generation_version().increment();

        let item = unsafe { get_handle!(&mut self.policy, *self.bucket.as_ref()) };

        self.policy.currsize = self
            .policy
            .currsize
            .saturating_sub(item.size())
            .saturating_add(new.size());

        std::mem::replace(item, new)
    }

    #[inline]
    fn remove(self) -> Self::Handle {
        self.shared.generation_version().increment();

        let (mut index, _) = unsafe { self.policy.table.remove(self.bucket) };
        index -= self.policy.front_offset;

        self.policy
            .decrement_indexes(index + 1, self.policy.entries.len());

        let handle = self.policy.entries.remove(index).unwrap();
        self.policy.currsize = self.policy.currsize.saturating_sub(handle.size());
        handle
    }
}

/// A view into a vacant slot in [`TTLPolicy`].
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut TTLPolicy,
    /// The shared configuration
    shared: &'a Shared,
}

impl traits::VacantExt for Vacant<'_> {
    type Handle = ExpiringHandle;
    type Shared = Shared;

    #[inline]
    fn would_exceed(&self, extra_size: usize) -> bool {
        self.policy.currsize.saturating_add(extra_size) > self.shared.maxsize()
    }

    #[inline]
    fn evict(&mut self) -> pyo3::PyResult<()> {
        self.policy.evict(self.shared)?;
        Ok(())
    }

    fn insert(self, handle: Self::Handle) {
        self.shared.generation_version().increment();

        self.policy.currsize = self.policy.currsize.saturating_add(handle.size());

        self.policy.table.insert(
            handle.key().hash(),
            self.policy.entries.len() + self.policy.front_offset,
            |index| get_handle!(&self.policy, *index).key().hash(),
        );
        self.policy.entries.push_back(handle);
    }
}

pub struct TTLPolicy {
    // Fields are same as `FIFOPolicy`
    table: hashbrown::raw::RawTable<usize>,
    entries: VecDeque<ExpiringHandle>,
    currsize: usize,
    front_offset: usize,
}

impl TTLPolicy {
    /// Creates a new [`TTLPolicy`].
    ///
    /// The underlying [`VecDeque`] is pre-allocated to hold at least `capacity` entries
    /// without reallocation.
    pub fn new(capacity: usize) -> Self {
        Self {
            table: hashbrown::raw::RawTable::with_capacity(capacity),
            entries: VecDeque::with_capacity(capacity),
            currsize: 0,
            front_offset: 0,
        }
    }

    #[inline]
    pub fn table(&self) -> &hashbrown::raw::RawTable<usize> {
        &self.table
    }

    #[inline]
    pub fn entries(&self) -> &VecDeque<ExpiringHandle> {
        &self.entries
    }

    #[inline]
    fn decrement_indexes(&mut self, start: usize, end: usize) {
        #[cfg(not(feature = "small-offset"))]
        const MAX_FRONT_OFFSET: usize = usize::MAX - isize::MAX as usize;

        #[cfg(feature = "small-offset")]
        const MAX_FRONT_OFFSET: usize = u8::MAX as usize;

        // Fast path: shifting the entire front is a single counter increment.
        // Guard against overflow; the full-normalization path below handles that case.
        if start <= 1 && end == self.entries.len() && self.front_offset < MAX_FRONT_OFFSET {
            self.front_offset += 1;
            return;
        }

        // Snapshot so the borrow checker doesn't complain about `self` inside the loops.
        let fo = self.front_offset;

        if (end - start) > self.table.num_buckets() / 2 {
            // Table-scan path: already O(n), so fold normalization in for free.
            // One pass: normalize every index (subtract fo) and decrement those in [start, end).
            unsafe {
                for bucket in self.table.iter() {
                    let i = bucket.as_mut();
                    let vd_idx = *i - fo; // raw VecDeque index
                    *i = if start <= vd_idx && vd_idx < end {
                        vd_idx - 1 // normalize + decrement
                    } else {
                        vd_idx // normalize only
                    };
                }
            }
        } else {
            // Entries-scan path: O(range) decrement pass, then O(n) normalization pass.
            //
            // Pass 1: decrement the logical indices for entries in [start, end).
            let shifted = self.entries.range(start..end);
            for (i, entry) in (start..end).zip(shifted) {
                let result = unsafe {
                    self.table
                        .get_mut(entry.key().hash(), |x| Ok::<_, pyo3::PyErr>((*x) - fo == i))
                        .unwrap_unchecked()
                };
                *result.expect("index not found") -= 1;
            }

            // Pass 2: normalize every stored index by subtracting `fo`.
            //   • Entries in  [start, end): (vd_idx + fo - 1) - fo  =  vd_idx - 1
            //   • All others:  (vd_idx + fo)     - fo               =  vd_idx
            if fo != 0 {
                unsafe {
                    for bucket in self.table.iter() {
                        *bucket.as_mut() -= fo;
                    }
                }
            }
        }

        // Both branches now store raw VecDeque indices, so the offset is zero.
        self.front_offset = 0;
    }

    pub fn expire(&mut self, gv: &utils::GenerationVersion) {
        let now = std::time::SystemTime::now();

        while let Some(handle) = self.entries.front() {
            if !handle.is_expired(now) {
                break;
            }

            let eq = |index: &usize| Ok::<_, pyo3::PyErr>((*index - self.front_offset) == 0);
            if std::hint::unlikely(
                self.table
                    .remove_entry(handle.key().hash(), eq)
                    .unwrap()
                    .is_none(),
            ) {
                unreachable!("popitem key not found in table");
            }

            gv.increment();

            let front = unsafe { self.entries.pop_front().unwrap_unchecked() };

            self.currsize = self.currsize.saturating_sub(front.size());
            self.decrement_indexes(1, self.entries.len());
        }
    }

    #[inline]
    pub fn iter(&mut self, shared: &Shared) -> utils::RawVecDequeIter<ExpiringHandle> {
        self.expire(shared.generation_version());

        let (first, second) = self.entries.as_slices();
        utils::RawVecDequeIter::new(first, second)
    }
}

impl PolicyExt for TTLPolicy {
    type Shared = Shared;
    type Handle = ExpiringHandle;

    type Occupied<'a>
        = Occupied<'a>
    where
        Self: 'a;

    type Vacant<'a>
        = Vacant<'a>
    where
        Self: 'a;

    #[inline]
    fn current_size(&self) -> usize {
        self.currsize
    }

    #[inline]
    fn get(
        &mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<Option<&Self::Handle>> {
        let eq = |index: &usize| get_handle!(&self, *index).key().py_eq(py, key);

        match self
            .table
            .get(key.hash(), eq)?
            .map(|index| get_handle!(&self, *index))
        {
            Some(handle) => {
                if handle.is_expired(std::time::SystemTime::now()) {
                    Ok(None)
                } else {
                    Ok(Some(handle))
                }
            }
            None => Ok(None),
        }
    }

    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>> {
        self.expire(shared.generation_version());

        let eq = |index: &usize| get_handle!(&self, *index).key().py_eq(py, key);
        match self.table.find(key.hash(), eq)? {
            Some(bucket) => {
                let result = Occupied {
                    policy: self,
                    shared,
                    bucket,
                };
                Ok(traits::PolicyEntry::Occupied(result))
            }
            None => {
                let result = Vacant {
                    policy: self,
                    shared,
                };
                Ok(traits::PolicyEntry::Vacant(result))
            }
        }
    }

    fn evict(&mut self, shared: &Self::Shared) -> pyo3::PyResult<Self::Handle> {
        let front = self.entries.pop_front();
        if front.is_none() {
            return Err(new_py_error!(PyKeyError, "cache is empty"));
        }

        let front = unsafe { front.unwrap_unchecked() };

        let eq = |index: &usize| Ok::<_, pyo3::PyErr>((*index - self.front_offset) == 0);
        if std::hint::unlikely(self.table.remove_entry(front.key().hash(), eq)?.is_none()) {
            unreachable!("popitem key not found in table");
        }

        shared.generation_version().increment();

        self.currsize = self.currsize.saturating_sub(front.size());
        self.decrement_indexes(1, self.entries.len());
        Ok(front)
    }

    #[inline]
    fn shrink_to_fit(&mut self, shared: &Self::Shared) {
        shared.generation_version().increment();

        self.table
            .shrink_to(0, |index| get_handle!(&self, *index).key().hash());
        self.entries.shrink_to_fit();
    }

    #[inline]
    fn clear(&mut self, shared: &Self::Shared) {
        if self.entries.is_empty() {
            return;
        }

        shared.generation_version().increment();
        self.table.clear();
        self.entries.clear();
        self.currsize = 0;
        self.front_offset = 0;
    }

    // TODO: considering expired handles
    fn py_eq(
        &self,
        py: pyo3::Python,
        shared: &Self::Shared,
        other: &Self,
        other_shared: &Self::Shared,
    ) -> pyo3::PyResult<bool> {
        if shared.maxsize() != other_shared.maxsize()
            || shared.global_ttl() != other_shared.global_ttl()
            || self.table.len() != other.table.len()
        {
            return Ok(false);
        }

        let mut error = None;
        let result = unsafe {
            let mut iterator = self.table.iter().map(|x| x.as_ref());

            iterator.all(|index_1| {
                let handle_1 = get_handle!(&self, *index_1);

                let result = other.table.get(handle_1.key().hash(), |index| {
                    handle_1.key().py_eq(py, get_handle!(&other, *index).key())
                });

                match result {
                    Err(e) => {
                        error = Some(e);
                        // Return false to break the `.all` loop
                        false
                    }
                    Ok(None) => false,
                    Ok(Some(index_2)) => {
                        let handle_2 = get_handle!(&other, *index_2);

                        let value_1 = handle_1.value();
                        let value_2 = handle_2.value();

                        match utils::pyobject_equal(py, value_1.as_ptr(), value_2.as_ptr()) {
                            Ok(result) => result,
                            Err(e) => {
                                error = Some(e);
                                // Return false to break the `.all` loop
                                false
                            }
                        }
                    }
                }
            })
        };

        if let Some(error) = error {
            return Err(error);
        }
        Ok(result)
    }

    fn clone_ref(&mut self, py: pyo3::Python<'_>) -> Self {
        let mut entries = VecDeque::with_capacity(self.entries.len());
        for handle in self.entries.iter() {
            entries.push_back(handle.clone_ref(py));
        }

        Self {
            table: self.table.clone(),
            entries,
            currsize: self.currsize,
            front_offset: self.front_offset,
        }
    }
}
