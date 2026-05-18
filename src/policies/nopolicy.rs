use crate::hashbrown;
use crate::internal::alias;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::PolicyExt;

/// A key-value pair with a precomputed hash and combined memory size.
///
/// The `size` field caches the result of `getsizeof(key) + getsizeof(value)`
/// so that [`NoPolicy`] can maintain an accurate `currsize` budget without
/// re-invoking the Python-side sizing function on every access.
pub struct Handle {
    /// The cache key together with its precomputed hash, avoiding repeated
    /// Python hash calls during table lookups.
    key: utils::PrecomputedHashObject,
    /// The cached value associated with this key.
    value: alias::PyObject,
    /// Combined memory footprint of the key and value as reported by `getsizeof`.
    size: usize,
}

impl Handle {
    /// Creates a new [`Handle`], which calculates the precomputed hash itself.
    pub fn new(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        Self::with_precomputed_hash_key(
            py,
            getsizeof,
            utils::PrecomputedHashObject::new(py, key)?,
            value,
        )
    }

    /// Creates a new [`Handle`] from an already-hashed key.
    ///
    /// Prefer this over [`Handle::new`] when the caller has already paid the cost
    /// of computing the hash (e.g. during a table lookup that preceded insertion).
    pub fn with_precomputed_hash_key(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        key: utils::PrecomputedHashObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        let size = getsizeof.call(py, key.as_ref(), &value)?;
        Ok(Self { key, value, size })
    }

    /// Consumes `self` and returns the [`utils::PrecomputedHashObject`].
    pub fn into_key(self) -> utils::PrecomputedHashObject {
        self.key
    }

    /// Returns a reference to the value.
    pub fn value(&self) -> &alias::PyObject {
        &self.value
    }

    /// Consumes `self` and returns the value of the pair.
    pub fn into_value(self) -> alias::PyObject {
        self.value
    }

    /// Consumes `self` and returns the pair.
    pub fn into_pair(self) -> (utils::PrecomputedHashObject, alias::PyObject) {
        (self.key.into(), self.value)
    }

    /// Makes a clone of self.
    ///
    /// This creates another pointer to the same object, increasing its reference count.
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self {
            key: self.key.clone_ref(py),
            value: self.value.clone_ref(py),
            size: self.size,
        }
    }
}

impl traits::HandleExt for Handle {
    type Key = utils::PrecomputedHashObject;

    fn key(&self) -> &utils::PrecomputedHashObject {
        &self.key
    }

    fn size(&self) -> usize {
        self.size
    }
}

/// A view into an occupied entry in [`NoPolicy`].
///
/// Holds a mutable reference to the parent policy and a raw bucket pointer
/// to the existing [`Handle`], enabling in-place removal or replacement without
/// an additional lookup.
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut NoPolicy,
    /// Raw bucket pointing to the occupied slot within the hash table.
    bucket: hashbrown::raw::Bucket<Handle>,
}

impl traits::EntryExt for Occupied<'_> {
    type Handle = Handle;

    fn would_exceed(&self, extra_size: usize) -> bool {
        let handle = unsafe { self.bucket.as_ref() };

        self.policy
            .currsize
            .saturating_add(extra_size)
            .saturating_sub(handle.size)
            >= self.policy.maxsize.get()
    }

    fn evict(&mut self) -> pyo3::PyResult<Self::Handle> {
        self.policy.evict()
    }
}

impl traits::OccupiedExt for Occupied<'_> {
    fn remove(self) -> Self::Handle {
        let (h, _) = unsafe { self.policy.table.remove(self.bucket) };
        self.policy.currsize = self.policy.currsize.saturating_sub(h.size);
        self.policy.gv.increment();
        h
    }

    fn replace(self, new: Self::Handle) -> Self::Handle {
        self.policy.currsize = self.policy.currsize.saturating_add(new.size);
        let old = unsafe { std::mem::replace(self.bucket.as_mut(), new) };
        self.policy.currsize = self.policy.currsize.saturating_sub(old.size);
        old
    }
}

/// A view into a vacant slot in [`NoPolicy`].
///
/// Holds a mutable reference to the parent policy, allowing a new [`Handle`]
/// to be inserted into the pre-located empty slot without a second lookup.
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut NoPolicy,
    /// If true, means we used `.evict()` method, and empty slots are available
    /// in table; so we don't need to reserve a new one.
    space_available: bool,
}

impl traits::EntryExt for Vacant<'_> {
    type Handle = Handle;

    fn would_exceed(&self, extra_size: usize) -> bool {
        self.policy.currsize.saturating_add(extra_size) >= self.policy.maxsize.get()
    }

    fn evict(&mut self) -> pyo3::PyResult<Self::Handle> {
        self.policy.evict()
    }
}

impl traits::VacantExt for Vacant<'_> {
    fn insert(self, handle: Self::Handle) {
        self.policy.currsize = self.policy.currsize.saturating_add(handle.size);

        if !self.space_available {
            self.policy.table.reserve(1, |x| x.key.hash());
        }
        unsafe {
            self.policy.table.insert_no_grow(handle.key.hash(), handle);
        }

        self.policy.gv.increment();
    }
}

/// A cache policy that performs **no eviction**.
///
/// Insertions are rejected once `currsize` would exceed `maxsize`; the caller
/// must free space manually or accept the refusal. This is useful when the
/// eviction strategy is handled externally, or when a hard size cap with no
/// silent data loss is desired.
pub struct NoPolicy {
    /// The raw hash table storing all live [`Handle`] entries.
    table: hashbrown::raw::RawTable<Handle>,
    /// Hard upper bound on `currsize`. Stored as [`NonZeroUsize`](std::num::NonZeroUsize)
    /// so the compiler can elide a zero-check branch in division/comparison hot paths.
    maxsize: std::num::NonZeroUsize,
    /// Running total of all stored handles' sizes, maintained incrementally.
    currsize: usize,
    /// Monotonically incrementing counter bumped on every structural mutation
    /// (insert, remove, clear, shrink). Used to detect iterator invalidation.
    gv: utils::GenerationVersion,
    /// Callable used to measure the memory footprint of each key-value pair.
    getsizeof: utils::GetsizeofFunction,
}

impl NoPolicy {
    /// Creates a new [`NoPolicy`] with the given initial `capacity` (number of slots)
    /// and a `maxsize` budget limit.
    ///
    /// The underlying hash table is pre-allocated to hold at least `capacity` entries
    /// without reallocation.
    #[inline]
    pub fn new(capacity: usize, maxsize: usize, getsizeof: Option<alias::PyObject>) -> Self {
        Self {
            table: hashbrown::raw::RawTable::with_capacity(capacity),
            maxsize: safe_non_zero!(maxsize),
            currsize: 0,
            gv: utils::GenerationVersion::default(),
            getsizeof: utils::GetsizeofFunction::new(getsizeof),
        }
    }

    /// Returns a reference to the underlying raw hash table.
    pub fn table(&self) -> &hashbrown::raw::RawTable<Handle> {
        &self.table
    }

    /// Returns a snapshot of the current [`utils::GenerationVersion`].
    ///
    /// Callers can compare a saved snapshot against a later call to detect
    /// whether the table was mutated in the interim.
    pub fn generation_version(&self) -> utils::GenerationVersion {
        self.gv.clone()
    }

    /// Returns a reference to the size-measuring function used during insertion.
    pub fn getsizeof(&self) -> &utils::GetsizeofFunction {
        &self.getsizeof
    }

    /// Makes a clone of `self`.
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        let mut table = hashbrown::raw::RawTable::with_capacity(self.table.capacity());

        unsafe {
            for handle in self.table.iter().map(|x| x.as_ref()) {
                table.insert_no_grow(handle.key.hash(), handle.clone_ref(py));
            }
        }

        Self {
            table,
            maxsize: self.maxsize,
            currsize: self.currsize,
            gv: utils::GenerationVersion::default(),
            getsizeof: self.getsizeof.clone_ref(py),
        }
    }
}

impl traits::PolicyExt for NoPolicy {
    type Handle = Handle;

    type Occupied<'a>
        = Occupied<'a>
    where
        Self: 'a;

    type Vacant<'a>
        = Vacant<'a>
    where
        Self: 'a;

    /// Returns the maximum allowed cumulative size of all stored entries.
    fn maxsize(&self) -> usize {
        self.maxsize.get()
    }

    /// Returns the current cumulative size of all stored entries.
    fn current_size(&self) -> usize {
        self.currsize
    }

    fn get(
        &mut self,
        py: pyo3::Python,
        key: &<Self::Handle as traits::HandleExt>::Key,
    ) -> pyo3::PyResult<Option<&Self::Handle>> {
        let bucket = self.table.find(key.hash(), |x| key.py_eq(py, &x.key))?;
        Ok(bucket.map(|x| unsafe { x.as_ref() }))
    }

    fn entry(
        &mut self,
        py: pyo3::Python,
        key: &<Self::Handle as traits::HandleExt>::Key,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'_>, Self::Vacant<'_>>> {
        match self.table.find(key.hash(), |x| key.py_eq(py, &x.key))? {
            Some(bucket) => {
                let result = Occupied {
                    policy: self,
                    bucket,
                };
                Ok(traits::PolicyEntry::Occupied(result))
            }
            None => {
                let result = Vacant {
                    policy: self,
                    space_available: false,
                };
                Ok(traits::PolicyEntry::Vacant(result))
            }
        }
    }

    fn evict(&mut self) -> pyo3::PyResult<Self::Handle> {
        Err(new_py_error!(
            PyNotImplementedError,
            "The cache has no algorithm to evict items"
        ))
    }

    fn shrink_to_fit(&mut self) {
        let initial = self.table.capacity();
        self.table.shrink_to(0, |x| x.key.hash());

        if initial != self.table.capacity() {
            self.gv.increment();
        }
    }

    fn clear(&mut self) {
        if self.table.is_empty() {
            return;
        }
        self.table.clear();
        self.gv.increment();
    }

    fn py_eq(&self, py: pyo3::Python, other: &Self) -> pyo3::PyResult<bool> {
        if self.maxsize() != other.maxsize() || self.table.len() != other.table.len() {
            return Ok(false);
        }

        let mut error = None;
        let result = unsafe {
            let mut iterator = self.table.iter().map(|x| x.as_ref());

            iterator.all(|handle_1| {
                let result = other
                    .table
                    .get(handle_1.key.hash(), |x| handle_1.key.py_eq(py, &x.key));

                match result {
                    Err(e) => {
                        error = Some(e);
                        // Return false to break the `.all` loop
                        false
                    }
                    Ok(None) => false,
                    Ok(Some(handle_2)) => {
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
}
