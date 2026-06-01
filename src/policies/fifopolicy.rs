use std::collections::VecDeque;

use crate::hashbrown;
use crate::internal::alias;
use crate::internal::pickle::Builder;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;

pub use super::common::Handle;
pub use super::common::Shared;

/// Shorthand for `self.entries[index - self.front_offset]`
macro_rules! get_handle {
    (&$slf:expr, $index:expr) => {
        &$slf.entries[$index - $slf.front_offset]
    };
    (&mut $slf:expr, $index:expr) => {
        &mut $slf.entries[$index - $slf.front_offset]
    };
}

/// A view into an occupied entry in [`FIFOPolicy`].
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut FIFOPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// Raw bucket pointing to the occupied index.
    bucket: hashbrown::raw::Bucket<usize>,
}

impl traits::OccupiedExt for Occupied<'_> {
    type Handle = Handle;
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

/// A view into a vacant slot in [`FIFOPolicy`].
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut FIFOPolicy,
    /// The shared configuration
    shared: &'a Shared,
}

impl traits::VacantExt for Vacant<'_> {
    type Handle = Handle;
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

pub struct FIFOPolicy {
    /// Maps each key to its logical index into [`FIFOPolicy::entries`], enabling O(1) lookups.
    ///
    /// Stored indices are *logical* (i.e. they do not reset when entries are popped from the
    /// front), so they must be adjusted on read: `entries[table[k] - front_offset]`.
    /// As a result, table values grow monotonically over the lifetime of the cache,
    /// but their *count* stays bounded by the cache capacity — this is not a memory concern.
    table: hashbrown::raw::RawTable<usize>,

    /// Insertion-ordered sequence of cached handles, providing O(1) front removal.
    entries: VecDeque<Handle>,

    /// Running total of all stored handles' sizes, maintained incrementally.
    currsize: usize,

    /// Number of handles ever popped from the front of [`FIFOPolicy::entries`].
    ///
    /// Because [`VecDeque`] indices shift on front-removal, naively keeping
    /// [`FIFOPolicy::table`] consistent would require decrementing every stored
    /// index — an O(n) operation. Instead, this counter is incremented on each
    /// pop and subtracted at read time: `entries[table[k] - front_offset]`,
    /// keeping both the pop and the lookup O(1).
    ///
    /// To prevent `usize` overflow in the subtraction, once `front_offset`
    /// reaches `usize::MAX - isize::MAX`, all indices in `table` are decremented
    /// by the current `front_offset` and the counter is reset to zero. This
    /// rewrite is O(n) but occurs so rarely, at most once per
    /// `usize::MAX - isize::MAX` evictions, that it is effectively free in practice.
    front_offset: usize,
}

impl FIFOPolicy {
    /// Creates a new [`FIFOPolicy`].
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
    pub fn entries(&self) -> &VecDeque<Handle> {
        &self.entries
    }

    #[inline]
    fn decrement_indexes(&mut self, start: usize, end: usize) {
        #[cfg(not(feature = "use-small-offset"))]
        const MAX_FRONT_OFFSET: usize = usize::MAX - isize::MAX as usize;

        #[cfg(feature = "use-small-offset")]
        const MAX_FRONT_OFFSET: usize = u8::MAX as usize;

        // Fast path: shifting the entire front is a single counter increment.
        // Guard against overflow; the full-normalization path below handles that case.
        if start <= 1 && end == self.entries.len() && self.front_offset < MAX_FRONT_OFFSET {
            self.front_offset += 1;
            return;
        }

        if (end - start) > self.table.capacity() / 2 {
            // Table-scan
            // normalize every index (subtract fo) and decrement those in range [start, end).
            unsafe {
                for bucket in self.table.iter() {
                    let i = bucket.as_mut();

                    let vd_idx = *i - self.front_offset;

                    *i = if start <= vd_idx && vd_idx < end {
                        vd_idx - 1 // normalize + decrement
                    } else {
                        vd_idx // normalize
                    };
                }
            }
        } else {
            // Entries-scan
            // decrement the logical indices for entries in range [start, end).
            let shifted = self.entries.range(start..end);
            for (i, entry) in (start..end).zip(shifted) {
                let result = unsafe {
                    self.table
                        .get_mut(entry.key().hash(), |x| {
                            Ok::<_, pyo3::PyErr>((*x) - self.front_offset == i)
                        })
                        .unwrap_unchecked()
                        .expect("index not found")
                };
                *result -= 1;
            }

            // normalize every stored index by subtracting `fo`.
            //   - Entries in  [start, end): (vd_idx + fo - 1) - fo  =  vd_idx - 1
            //   - All others:               (vd_idx + fo)     - fo  =  vd_idx
            if self.front_offset != 0 {
                unsafe {
                    for bucket in self.table.iter() {
                        *bucket.as_mut() -= self.front_offset;
                    }
                }
            }
        }

        // Both branches now store raw VecDeque indices, so the offset is zero.
        self.front_offset = 0;
    }

    #[inline]
    pub fn iter(&self) -> utils::RawVecDequeIter<Handle> {
        let (first, second) = self.entries.as_slices();
        utils::RawVecDequeIter::new(first, second)
    }
}

impl PolicyExt for FIFOPolicy {
    type Shared = Shared;
    type Handle = Handle;

    type Occupied<'a>
        = Occupied<'a>
    where
        Self: 'a;

    type Vacant<'a>
        = Vacant<'a>
    where
        Self: 'a;

    const PICKLE_SIZE: usize = 1;

    #[inline]
    fn current_size(&self) -> usize {
        self.currsize
    }

    #[inline]
    fn get(
        &mut self,
        py: pyo3::Python,
        key: &<Self::Handle as traits::HandleExt>::Key,
    ) -> pyo3::PyResult<Option<&Self::Handle>> {
        let eq = |index: &usize| get_handle!(&self, *index).key().py_eq(py, key);
        match self.table.get(key.hash(), eq)? {
            Some(index) => Ok(Some(get_handle!(&self, *index))),
            None => Ok(None),
        }
    }

    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>> {
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
        let front = self.entries.front();
        if front.is_none() {
            return Err(new_py_error!(PyKeyError, ()));
        }

        let front = unsafe { front.unwrap_unchecked() };

        let eq = |index: &usize| Ok::<_, pyo3::PyErr>(*index - self.front_offset == 0);
        if std::hint::unlikely(self.table.remove_entry(front.key().hash(), eq)?.is_none()) {
            unreachable!("popitem key not found in table");
        }

        shared.generation_version().increment();

        self.decrement_indexes(1, self.entries.len());
        let front = unsafe { self.entries.pop_front().unwrap_unchecked() };

        self.currsize = self.currsize.saturating_sub(front.size());

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

    fn py_eq(
        &self,
        py: pyo3::Python,
        shared: &Self::Shared,
        other: &Self,
        other_shared: &Self::Shared,
    ) -> pyo3::PyResult<bool> {
        if shared.maxsize() != other_shared.maxsize() || self.table.len() != other.table.len() {
            return Ok(false);
        }

        let mut error = None;

        let result = unsafe {
            self.table.iter().all(|x| {
                let handle = get_handle!(&self, *x.as_ref());
                let key = handle.key();

                match other
                    .table
                    .get(key.hash(), |i| key.py_eq(py, get_handle!(&other, *i).key()))
                {
                    Err(e) => {
                        error = Some(e);
                        false
                    }
                    Ok(None) => false,
                    Ok(Some(i)) => {
                        let v1 = handle.value();
                        let v2 = get_handle!(&other, *i).value();
                        match utils::pyobject_equal(py, v1.as_ptr(), v2.as_ptr()) {
                            Ok(eq) => eq,
                            Err(e) => {
                                error = Some(e);
                                false
                            }
                        }
                    }
                }
            })
        };

        error.map_or(Ok(result), Err)
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

    fn build_pickle(
        &self,
        tuple: &mut crate::internal::pickle::TupleBuilder<
            '_,
            crate::internal::pickle::PickleBuilder,
        >,
    ) -> pyo3::PyResult<()> {
        let mut list = tuple.begin_list()?;

        for handle in self.entries.iter() {
            let mut tuple = list.begin_tuple(2)?;
            tuple.push(handle.key().as_ref())?;
            tuple.push(handle.value())?;
            tuple.end()?;
        }

        list.end()
    }

    fn from_pickle(
        maxsize: usize,
        getsizeof: Option<crate::internal::alias::PyObject>,
        _global_ttl: Option<std::time::Duration>,
        builded: pyo3::Bound<'_, pyo3::types::PyTuple>,
    ) -> pyo3::PyResult<(Self::Shared, Self)> {
        use pyo3::types::PyAnyMethods;
        use pyo3::types::PyListMethods;
        use pyo3::types::PyTupleMethods;

        let list = builded.get_item(0)?.cast_into::<pyo3::types::PyList>()?;
        let list_length = list.len();

        if list_length > maxsize {
            return Err(new_py_error!(
                PyValueError,
                "list size is incompatible with maxsize"
            ));
        }

        let shared = Shared::new(maxsize, getsizeof);
        let mut slf = Self::new(list.len());

        for bound in list.iter() {
            let (key, value) = bound.extract::<(alias::PyObject, alias::PyObject)>()?;

            let handle = Handle::new(bound.py(), shared.getsizeof(), key, value)?;

            slf.currsize = slf.currsize.saturating_add(handle.size());

            unsafe {
                slf.table.insert_no_grow(
                    handle.key().hash(),
                    // Adding `slf.front_offset` is unnecessary here
                    slf.entries.len(),
                );
            }
            slf.entries.push_back(handle);
        }

        Ok((shared, slf))
    }
}
