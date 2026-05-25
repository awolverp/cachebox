use crate::hashbrown;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;

pub use super::common::Handle;
pub use super::common::Shared;

/// A view into an occupied entry in [`NoPolicy`].
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut NoPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// Raw bucket pointing to the occupied slot within the hash table.
    bucket: hashbrown::raw::Bucket<Handle>,
}

impl traits::OccupiedExt for Occupied<'_> {
    type Shared = Shared;
    type Handle = Handle;

    fn remove(self) -> Self::Handle {
        self.shared.generation_version().increment();

        let (h, _) = unsafe { self.policy.table.remove(self.bucket) };
        self.policy.currsize = self.policy.currsize.saturating_sub(h.size());
        h
    }

    fn replace(self, new: Self::Handle) -> Self::Handle {
        self.policy.currsize = self.policy.currsize.saturating_add(new.size());
        let old = unsafe { std::mem::replace(self.bucket.as_mut(), new) };
        self.policy.currsize = self.policy.currsize.saturating_sub(old.size());

        old
    }
}

/// A view into a vacant slot in [`NoPolicy`].
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut NoPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// If true, means we used `.evict()` method, and empty slots are available
    /// in table; so we don't need to reserve a new one.
    space_available: bool,
}

impl traits::VacantExt for Vacant<'_> {
    type Shared = Shared;
    type Handle = Handle;

    #[inline]
    fn would_exceed(&self, extra_size: usize) -> bool {
        self.policy.currsize.saturating_add(extra_size) > self.shared.maxsize()
    }

    #[inline(always)]
    fn evict(&mut self, py: pyo3::Python) -> pyo3::PyResult<()> {
        self.policy.evict(py, self.shared)?;
        Ok(())
    }

    fn insert(self, handle: Self::Handle) {
        self.shared.generation_version().increment();
        self.policy.currsize = self.policy.currsize.saturating_add(handle.size());

        if !self.space_available {
            self.policy.table.reserve(1, |x| x.key().hash());
        }
        unsafe {
            self.policy
                .table
                .insert_no_grow(handle.key().hash(), handle);
        }
    }
}

pub struct NoPolicy {
    /// The raw hash table storing all live [`Handle`] entries.
    table: hashbrown::raw::RawTable<Handle>,
    /// Running total of all stored handles' sizes, maintained incrementally.
    currsize: usize,
}

impl NoPolicy {
    /// Creates a new [`NoPolicy`].
    ///
    /// The underlying hash table is pre-allocated to hold at least `capacity` entries
    /// without reallocation.
    pub fn new(capacity: usize) -> Self {
        Self {
            table: hashbrown::raw::RawTable::with_capacity(capacity),
            currsize: 0,
        }
    }

    /// Returns a reference to the underlying raw hash table.
    #[inline(always)]
    pub fn table(&self) -> &hashbrown::raw::RawTable<Handle> {
        &self.table
    }
}

impl traits::PolicyExt for NoPolicy {
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
        let bucket = self.table.find(key.hash(), |x| key.py_eq(py, x.key()))?;
        Ok(bucket.map(|x| unsafe { x.as_ref() }))
    }

    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as traits::HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>> {
        match self.table.find(key.hash(), |x| key.py_eq(py, x.key()))? {
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
                    space_available: false,
                };
                Ok(traits::PolicyEntry::Vacant(result))
            }
        }
    }

    #[inline]
    fn evict(&mut self, _py: pyo3::Python, _shared: &Self::Shared) -> pyo3::PyResult<Self::Handle> {
        Err(new_py_error!(
            PyOverflowError,
            "The cache has no algorithm to evict items"
        ))
    }

    #[inline]
    fn shrink_to_fit(&mut self, shared: &Self::Shared) {
        shared.generation_version().increment();
        self.table.shrink_to(0, |x| x.key().hash());
    }

    #[inline]
    fn clear(&mut self, shared: &Self::Shared) {
        if self.table.is_empty() {
            return;
        }
        self.table.clear();
        shared.generation_version().increment();
        self.currsize = 0;
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
            let mut iterator = self.table.iter().map(|x| x.as_ref());

            iterator.all(|handle_1| {
                let result = other
                    .table
                    .get(handle_1.key().hash(), |x| handle_1.key().py_eq(py, x.key()));

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

    fn clone_ref(&mut self, py: pyo3::Python<'_>) -> Self {
        let mut table = hashbrown::raw::RawTable::with_capacity(self.table.capacity());

        unsafe {
            for handle in self.table.iter().map(|x| x.as_ref()) {
                table.insert_no_grow(handle.key().hash(), handle.clone_ref(py));
            }
        }

        Self {
            table,
            currsize: self.currsize,
        }
    }
}
