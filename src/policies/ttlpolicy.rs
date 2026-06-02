use std::collections::VecDeque;

use crate::hashbrown;
use crate::internal::alias;
use crate::internal::pickle::Builder;
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
        expires_at: utils::ExpiresAt,
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
        expires_at: utils::ExpiresAt,
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
    const PICKLE_SIZE: usize = 1;

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
        let now = std::time::SystemTime::now();

        let result = unsafe {
            self.table.iter().all(|x| {
                let handle = get_handle!(&self, *x.as_ref());
                if handle.is_expired(now) {
                    return true;
                }

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
            let mut tuple = list.begin_tuple(3)?;
            tuple.push(handle.key().as_ref())?;
            tuple.push(handle.value())?;
            tuple.push(
                handle
                    .expires_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap(),
            )?;
            tuple.end()?;
        }

        list.end()
    }

    fn from_pickle(
        maxsize: usize,
        getsizeof: Option<crate::internal::alias::PyObject>,
        global_ttl: Option<std::time::Duration>,
        builded: pyo3::Bound<'_, pyo3::types::PyTuple>,
    ) -> pyo3::PyResult<(Self::Shared, Self)> {
        use pyo3::types::PyAnyMethods;
        use pyo3::types::PyListMethods;
        use pyo3::types::PyTupleMethods;

        if global_ttl.is_none_or(|x| x.is_zero()) {
            return Err(new_py_error!(PyValueError, "global_ttl is zero"));
        }

        let list = builded.get_item(0)?.cast_into::<pyo3::types::PyList>()?;
        let list_length = list.len();

        if list_length > maxsize {
            return Err(new_py_error!(
                PyValueError,
                "list size is incompatible with maxsize"
            ));
        }

        let shared = Shared::with_ttl(maxsize, getsizeof, global_ttl);
        let mut slf = Self::new(list.len());

        for bound in list.iter() {
            let (key, value, timestamp) =
                bound.extract::<(alias::PyObject, alias::PyObject, f64)>()?;

            let handle = ExpiringHandle::new(
                bound.py(),
                shared.getsizeof(),
                (std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(timestamp)).into(),
                key,
                value,
            )?;

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
