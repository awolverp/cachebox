use std::ptr::NonNull;

use crate::internal::utils;

/// A collection that defers sorting until an ordered operation is requested.
///
/// Unlike a classic binary heap, `LazyHeap` does not maintain a heap
/// invariant after every insertion. Instead it tracks a dirty flag and
/// re-sorts the entire backing buffer the first time an ordered operation is
/// needed. This amortises well when many insertions occur before any removal,
/// because one `O(n log n)` sort is cheaper than repeated `O(log n)` sift-ups.
///
/// # Ownership model
/// `LazyHeap<T>` is the **sole owner** of every element it holds. Cursors are
/// purely non-owning handles and must never be used to free the backing
/// allocation.
pub struct LazyHeap<T> {
    data: std::collections::VecDeque<NonNull<T>>,
    is_sorted: bool,
    _marker: std::marker::PhantomData<Box<T>>,
}

impl<T> LazyHeap<T> {
    /// Pops and owns the front allocation. Does **not** sort.
    #[inline]
    fn unlink_front(&mut self) -> Option<T> {
        let ptr = self.data.pop_front()?;
        // SAFETY: LazyHeap owns the sole Box for every pointer it stores.
        Some(*unsafe { Box::from_raw(ptr.as_ptr()) })
    }

    /// Pops and owns the back allocation. Does **not** sort.
    #[inline]
    fn unlink_back(&mut self) -> Option<T> {
        let ptr = self.data.pop_back()?;
        // SAFETY: LazyHeap owns the sole Box for every pointer it stores.
        Some(*unsafe { Box::from_raw(ptr.as_ptr()) })
    }
}

impl<T> LazyHeap<T> {
    /// Creates a new, empty `LazyHeap`.
    pub fn new() -> Self {
        Self {
            data: std::collections::VecDeque::new(),
            is_sorted: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the number of elements in the heap.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the heap contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Inserts `value` into the heap and returns a [`Cursor`] to it.
    ///
    /// The returned cursor is **non-owning**. Store it in an external structure
    /// (e.g. a `hashbrown::RawTable`) for later removal via [`remove`](Self::remove).
    /// Never reconstruct a `Box` from it.
    ///
    /// This call marks the heap as unsorted; the next ordered operation
    /// triggers a full sort.
    ///
    /// # Complexity
    /// Amortised O(1).
    #[inline]
    pub fn push(&mut self, value: T) -> Cursor<T> {
        // SAFETY: Box::into_raw is guaranteed non-null.
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(value))) };
        self.data.push_back(ptr);
        self.is_sorted = false;
        Cursor(ptr)
    }

    /// Marks the heap's order as invalid without re-sorting immediately.
    ///
    /// Call this after mutating an element's sort key through [`Cursor::as_mut`].
    /// The next ordered operation will then re-sort before proceeding.
    #[inline]
    pub fn mark_unsorted(&mut self) {
        self.is_sorted = false;
    }

    /// Sorts the backing buffer with `compare` if it is not already sorted and
    /// then returns `true`.
    ///
    /// All ordered operations call this automatically. You can call it
    /// manually to amortise the sort cost before a batch of [`front`](Self::front) /
    /// [`get`](Self::get) accesses.
    ///
    /// # Complexity
    /// O(n log n) when unsorted; O(1) when already sorted.
    #[inline]
    pub fn sort_by(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> bool {
        if self.is_sorted {
            return false;
        }
        if self.data.len() > 1 {
            // SAFETY: every pointer in `self.data` is a live, heap-owned allocation.
            unsafe {
                self.data
                    .make_contiguous()
                    .sort_by(|a, b| compare(a.as_ref(), b.as_ref()));
            }
        }
        self.is_sorted = true;
        true
    }

    /// Returns a cursor to the smallest (front) element without removing it,
    /// or `None` if the heap is empty.
    ///
    /// Sorts the heap first if necessary.
    #[inline]
    pub fn front(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Option<Cursor<T>> {
        self.sort_by(compare);
        self.data.front().copied().map(Cursor)
    }

    /// Returns a cursor to the largest (back) element without removing it,
    /// or `None` if the heap is empty.
    ///
    /// Sorts the heap first if necessary.
    #[inline]
    pub fn back(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Option<Cursor<T>> {
        self.sort_by(compare);
        self.data.back().copied().map(Cursor)
    }

    /// Returns a cursor to the element at position `index`, or `None` if out
    /// of bounds.
    ///
    /// The index is only meaningful after the heap has been sorted — consider
    /// calling [`sort_by`](Self::sort_by) first.
    #[inline]
    pub fn get(&self, index: usize) -> Option<Cursor<T>> {
        self.data.get(index).copied().map(Cursor)
    }

    /// Removes and returns the smallest (front) element, or `None` if empty.
    ///
    /// Sorts the heap first if necessary.
    ///
    /// # Complexity
    /// O(n log n) when unsorted; O(n) when already sorted (front removal from
    /// a `VecDeque` shifts elements).
    #[inline]
    pub fn pop_front(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Option<T> {
        self.sort_by(compare);
        self.unlink_front()
    }

    /// Removes and returns the largest (back) element, or `None` if empty.
    ///
    /// Sorts the heap first if necessary.
    ///
    /// # Complexity
    /// O(n log n) when unsorted; O(1) when already sorted.
    #[inline]
    pub fn pop_back(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Option<T> {
        self.sort_by(compare);
        self.unlink_back()
    }

    /// Removes and returns the element identified by `cursor`.
    ///
    /// Sorts the heap first if necessary, then performs a linear scan to
    /// locate the element by pointer identity.
    ///
    /// # Complexity
    /// O(n log n) when unsorted; O(n) when already sorted.
    pub fn remove(
        &mut self,
        cursor: Cursor<T>,
        compare: impl Fn(&T, &T) -> std::cmp::Ordering,
    ) -> T {
        debug_assert!(!self.data.is_empty());

        // Fast path: single element — no need to sort or scan.
        if self.data.len() == 1 {
            return self.unlink_back().unwrap();
        }

        self.sort_by(compare);

        let index = self
            .data
            .iter()
            .position(|ptr| cursor.0 == *ptr)
            .expect("cursor does not belong to this LazyHeap");

        // SAFETY: `index` was just returned by `position`, so it is in bounds.
        // LazyHeap holds the sole Box for this pointer; the cursor is non-owning.
        let ptr = unsafe { self.data.remove(index).unwrap_unchecked() };
        *unsafe { Box::from_raw(ptr.as_ptr()) }
    }

    /// Returns an iterator that yields a [`Cursor`] for each element in sorted
    /// order.
    ///
    /// Sorts the heap first if necessary. The returned [`Iter`] holds raw
    /// pointers into the backing buffer; do not mutate or drop the heap while
    /// it is alive.
    #[inline]
    pub fn iter(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> RawIter<T> {
        self.sort_by(compare);
        let (a, b) = self.data.as_slices();
        RawIter {
            first: utils::RawSliceIter::new(a),
            second: utils::RawSliceIter::new(b),
        }
    }

    /// Removes all elements, dropping each one.
    ///
    /// The heap is empty and considered sorted after this call.
    #[inline]
    pub fn clear(&mut self) {
        while self.unlink_back().is_some() {}
        self.is_sorted = true;
    }

    /// Shrinks the backing buffer's capacity as close to its current length
    /// as possible.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}

impl<T> Default for LazyHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<#[may_dangle] T> Drop for LazyHeap<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T>(&'a mut LazyHeap<T>);

        impl<'a, T> Drop for DropGuard<'a, T> {
            fn drop(&mut self) {
                // Continue the same loop we do below. This only runs when a destructor has
                // panicked. If another one panics this will abort.
                while self.0.unlink_back().is_some() {}
            }
        }

        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard(self);
        while guard.0.unlink_back().is_some() {}
        std::mem::forget(guard);
    }
}

/// A non-owning, pointer-sized handle to an element stored in a [`LazyHeap`].
///
/// Think of `Cursor<T>` as a stable address you can cache in an external data
/// structure (e.g. `hashbrown::raw::RawTable`) and later hand back to
/// [`LazyHeap::remove`] for cheap lookup and removal. It carries **no
/// ownership**: every allocation is owned exclusively by the heap that
/// produced the cursor.
///
/// Using a stale cursor is undefined behaviour.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Cursor<T>(NonNull<T>);

impl<T> Cursor<T> {
    /// Returns a shared reference to the value this cursor points to.
    ///
    /// # Safety
    /// The cursor must be valid (see the [type-level docs](Self)).
    #[inline]
    pub unsafe fn element(&self) -> &T {
        self.0.as_ref()
    }

    /// Returns a mutable reference to the value this cursor points to.
    ///
    /// If the mutation changes any field that affects sort order, you **must**
    /// call [`LazyHeap::invalidate`] afterwards so the heap re-sorts before
    /// the next ordered operation.
    ///
    /// # Safety
    /// - The cursor must be valid (see the [type-level docs](Self)).
    /// - No other reference to the same element may be alive simultaneously.
    #[inline]
    pub unsafe fn element_mut(&mut self) -> &mut T {
        self.0.as_mut()
    }

    /// Returns the raw pointer underlying this cursor.
    ///
    /// Prefer [`as_ref`](Self::as_ref) or [`as_mut`](Self::as_mut) for
    /// element access. This exists for interoperability with APIs that require
    /// a raw pointer (e.g. hashing into a `RawTable` by address).
    ///
    /// **Never** reconstruct a `Box` from this pointer — doing so transfers
    /// ownership out of the heap and causes a double-free.
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
}

/// Raw iterator for [`VecDeque`] which doesn't have lifetime.
///
/// # Safety
/// You should track changes of [`VecDeque`] yourself.
pub struct RawIter<T> {
    first: utils::RawSliceIter<NonNull<T>>,
    second: utils::RawSliceIter<NonNull<T>>,
}

impl<T> Iterator for RawIter<T> {
    type Item = Cursor<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.first.next() {
            Some(val) => Some(
                // SAFETY: `val` is a valid `NonNull<NonNull<T>>` pointing into the
                // first slice of the `VecDeque`. The pointee is `Copy` and remains
                // valid as long as the `VecDeque` is alive and unmodified, which the
                // caller is required to uphold per this type's safety contract.
                Cursor(unsafe { val.read() }),
            ),
            None => {
                std::mem::swap(&mut self.first, &mut self.second);
                // SAFETY: same as above.
                self.first.next().map(|val| Cursor(unsafe { val.read() }))
            }
        }
    }
}

unsafe impl<T: Send + Send> Send for LazyHeap<T> {}
unsafe impl<T: Sync + Sync> Sync for LazyHeap<T> {}
unsafe impl<T: Send + Send> Send for RawIter<T> {}
unsafe impl<T: Sync + Sync> Sync for RawIter<T> {}
unsafe impl<T: Send + Send> Send for Cursor<T> {}
unsafe impl<T: Sync + Sync> Sync for Cursor<T> {}
