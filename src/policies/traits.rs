use crate::internal::utils;

pub trait HandleExt {
    type Key;

    /// Borrows the key stored in this handle.
    fn key(&self) -> &Self::Key;

    /// The size this handle contributes toward [`PolicyExt::maxsize`].
    ///
    /// Return `1` for count-based policies or a byte/cost value for
    /// size-based policies. Must be `> 0`.
    fn size(&self) -> usize;
}

/// Shared behaviour for occupied and vacant entry guards.
///
/// Both variants hold a mutable borrow of the parent policy, so budget checks
/// and eviction go through the entry rather than through the policy directly.
pub trait EntryExt {
    type Shared: SharedExt;
    type Handle: HandleExt;

    /// Returns `true` if adding `extra_size` would meet or exceed
    /// [`PolicyExt::weight_limit`].
    ///
    /// Call this *before* [`OccupiedExt::replace`] or [`VacantExt::insert`].
    fn would_exceed(&self, extra_size: usize) -> bool;

    /// Evicts one entry, freeing budget for a subsequent insert or replace.
    ///
    /// # Errors
    ///
    /// Returns any Python exception raised while dropping the evicted value.
    fn evict(&mut self) -> pyo3::PyResult<Self::Handle>;
}

/// Guard for an *occupied* slot.
pub trait OccupiedExt: EntryExt {
    /// Replaces the current handle with `new`, returning the old one.
    ///
    /// Does **not** enforce the weight budget; call
    /// [`would_exceed`](EntryExt::would_exceed) first.
    fn replace(self, new: Self::Handle) -> Self::Handle;

    /// Removes the handle from this slot and returns it.
    fn remove(self) -> Self::Handle;
}

/// Guard for a *vacant* slot.
pub trait VacantExt: EntryExt {
    /// Inserts `handle` into this slot.
    ///
    /// Does **not** enforce the weight budget; call
    /// [`would_exceed`](EntryExt::would_exceed) first.
    fn insert(self, handle: Self::Handle);
}

/// The state of a policy slot, returned by [`PolicyExt::entry`].
pub enum PolicyEntry<O, V> {
    Occupied(O),
    Vacant(V),
}

pub trait SharedExt: Send + Sync {
    /// Returns the configured maxsize.
    fn maxsize(&self) -> usize;

    /// Returns the current total cumulative size consumed by all stored entries.
    fn current_size(&self) -> usize;

    /// Returns the generation version.
    fn generation_version(&self) -> utils::GenerationVersion;

    /// Returns a reference to configued getsizeof function.
    fn getsizeof(&self) -> &utils::GetsizeofFunction;

    /// Make a clone of `self`.
    fn clone_ref(&self, py: pyo3::Python) -> Self;
}

pub trait PolicyExt {
    /// Read-only variables, we keep this type separated from the main policy implementation,
    /// because we need to access them outside of `Mutex`s.
    type Shared: SharedExt;
    type Handle: HandleExt;

    type Occupied<'a>: OccupiedExt<Handle = Self::Handle, Shared = Self::Shared> + 'a
    where
        Self: 'a;

    type Vacant<'a>: VacantExt<Handle = Self::Handle, Shared = Self::Shared> + 'a
    where
        Self: 'a;

    /// Looks up a handle by `hash` and `eq`, applying policy side-effects on hit.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `eq` raises a Python exception.
    fn get(
        &mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &Self::Shared,
    ) -> pyo3::PyResult<Option<&Self::Handle>>;

    /// Returns a [`PolicyEntry`] for the slot at `hash` / `eq`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `eq` raises a Python exception.
    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>>;

    /// Evicts a handle according to the policy algorithm, returning it.
    ///
    /// # Errors
    ///
    /// Returns `Err` if dropping the evicted value raises a Python exception.
    ///
    /// # Panics
    ///
    /// May panic if the policy is empty.
    fn evict(&mut self, shared: &Self::Shared) -> pyo3::PyResult<Self::Handle>;

    /// Removes all handles without shrinking the allocation.
    fn clear(&mut self, shared: &Self::Shared);

    /// Shrinks the internal allocation as close to length as possible.
    fn shrink_to_fit(&mut self, shared: &Self::Shared);

    /// Performs Python `==`.
    fn py_eq(
        &self,
        py: pyo3::Python,
        shared: &Self::Shared,
        other: &Self,
        other_shared: &Self::Shared,
    ) -> pyo3::PyResult<bool>;

    /// Make a clone of `self`.
    fn clone_ref(&self, py: pyo3::Python) -> Self;
}
