use std::ptr::NonNull;

use crate::internal::utils;

/// Bookkeeping stored inside every element of a [`LazyHeap`].
///
/// * `pos` — current index in the heap buffer, so removal by cursor is O(log n)
///   instead of a linear scan.
/// * `seq` — insertion sequence number, used as a tie-breaker so elements with
///   equal priority keep insertion order (same behaviour as the old stable sort).
/// * `rec` — the priority the element had when it was last placed. The heap
///   invariant is maintained on `(rec, seq)`, not on the live key. This lets a
///   policy *increase* an element's priority in O(1) (e.g. LFU `get`) without
///   touching the buffer layout; the stale root is repaired lazily in
///   [`LazyHeap::front`].
#[derive(Clone, Copy)]
pub struct HeapSlot<K> {
    seq: u64,
    rec: K,
    pos: u32,
    /// Index in [`LazyHeap::pending`], or [`NOT_PENDING`].
    pidx: u32,
}

const NOT_PENDING: u32 = u32::MAX;

impl<K: Default> Default for HeapSlot<K> {
    fn default() -> Self {
        Self {
            pos: 0,
            seq: 0,
            rec: K::default(),
            pidx: NOT_PENDING,
        }
    }
}

/// An element that can live in a [`LazyHeap`].
pub trait HeapNode {
    /// Priority type. Smaller is closer to the front.
    type Key: Ord + Copy;

    /// The element's *current* priority.
    fn heap_key(&self) -> Self::Key;
    fn heap_slot(&self) -> &HeapSlot<Self::Key>;
    fn heap_slot_mut(&mut self) -> &mut HeapSlot<Self::Key>;
}

/// An indexed binary min-heap with lazy priority increases and on-demand sorting.
///
/// Complexity:
/// * `push`, `pop_front`, `remove`, `update` — O(log n)
/// * `front` — O(1), plus amortised repair work paid for by earlier lazy increases
/// * `sort_by` / `iter` — O(n log n) only when the order is not already sorted
///
/// A fully sorted buffer is a valid heap, so ordered iteration simply sorts the
/// buffer in place and keeps using it as a heap afterwards.
///
/// # Lazy increases
/// A caller may increase an element's key through [`Cursor::element_mut`] without
/// telling the heap, as long as it never *decreases* it that way (use
/// [`mark_changed`](Self::mark_changed) for arbitrary changes). Because recorded keys never
/// exceed live keys and the heap is ordered on recorded keys, a root whose
/// recorded key equals its live key is the true minimum.
///
/// # Ownership model
/// `LazyHeap<T>` is the **sole owner** of every element it holds. Cursors are
/// purely non-owning handles and must never be used to free the backing
/// allocation.
pub struct LazyHeap<T: HeapNode> {
    data: Vec<NonNull<T>>,
    /// Elements whose key changed arbitrarily (possibly decreased) and still need
    /// re-positioning. Applied in [`front`](Self::front), so the buffer layout never
    /// moves under a live iterator because of a value replacement.
    pending: Vec<NonNull<T>>,
    next_seq: u64,
    is_sorted: bool,
    _marker: std::marker::PhantomData<Box<T>>,
}

impl<T: HeapNode> LazyHeap<T> {
    /// Creates a new, empty `LazyHeap`.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            pending: Vec::new(),
            next_seq: 0,
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

    #[inline(always)]
    unsafe fn node(&self, i: usize) -> &T {
        unsafe { self.data.get_unchecked(i).as_ref() }
    }

    #[inline(always)]
    unsafe fn node_mut(&mut self, i: usize) -> &mut T {
        unsafe { self.data.get_unchecked_mut(i).as_mut() }
    }

    /// `(rec, seq)` ordering between two buffer indices.
    #[inline(always)]
    unsafe fn less(&self, a: usize, b: usize) -> bool {
        unsafe {
            let (sa, sb) = (self.node(a).heap_slot(), self.node(b).heap_slot());
            (sa.rec, sa.seq) < (sb.rec, sb.seq)
        }
    }

    #[inline(always)]
    unsafe fn swap(&mut self, a: usize, b: usize) {
        self.data.swap(a, b);
        unsafe {
            self.node_mut(a).heap_slot_mut().pos = a as u32;
            self.node_mut(b).heap_slot_mut().pos = b as u32;
        }
    }

    /// Returns `true` if the element moved.
    unsafe fn sift_up(&mut self, mut i: usize) -> bool {
        let start = i;
        while i > 0 {
            let parent = (i - 1) / 2;
            if unsafe { !self.less(i, parent) } {
                break;
            }
            unsafe { self.swap(i, parent) };
            i = parent;
        }
        i != start
    }

    /// Returns `true` if the element moved.
    unsafe fn sift_down(&mut self, mut i: usize) -> bool {
        let start = i;
        let len = self.data.len();
        loop {
            let left = 2 * i + 1;
            if left >= len {
                break;
            }
            let right = left + 1;
            let child = if right < len && unsafe { self.less(right, left) } {
                right
            } else {
                left
            };
            if unsafe { !self.less(child, i) } {
                break;
            }
            unsafe { self.swap(i, child) };
            i = child;
        }
        i != start
    }

    /// Moves the element at `i` to its correct place after its recorded key changed.
    #[inline]
    unsafe fn fix(&mut self, i: usize) -> bool {
        unsafe { self.sift_up(i) || self.sift_down(i) }
    }

    /// Restores the invariant at the root after lazy increases, so the root is the
    /// true minimum. Returns `true` if the layout changed.
    fn repair_root(&mut self) -> bool {
        let mut moved = false;
        while !self.data.is_empty() {
            let root = unsafe { self.node_mut(0) };
            let key = root.heap_key();
            let slot = root.heap_slot_mut();
            if slot.rec == key {
                break;
            }
            slot.rec = key;
            if !unsafe { self.sift_down(0) } {
                break;
            }
            moved = true;
        }
        moved
    }

    /// Inserts `value` into the heap and returns a [`Cursor`] to it.
    ///
    /// The returned cursor is **non-owning**. Never reconstruct a `Box` from it.
    ///
    /// # Complexity
    /// O(log n).
    #[inline]
    pub fn push(&mut self, mut value: T) -> Cursor<T> {
        let pos = self.data.len();
        assert!(pos < NOT_PENDING as usize, "LazyHeap: too many elements");
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);

        let key = value.heap_key();
        *value.heap_slot_mut() = HeapSlot {
            pos: pos as u32,
            seq,
            rec: key,
            pidx: NOT_PENDING,
        };

        // SAFETY: Box::into_raw is guaranteed non-null.
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(value))) };
        self.data.push(ptr);

        // Appending the largest seq with a key >= the current last element keeps
        // a sorted buffer sorted (common for monotonic keys like expiry times).
        if self.is_sorted && pos > 0 {
            self.is_sorted = unsafe { !self.less(pos, pos - 1) };
        }
        unsafe { self.sift_up(pos) };
        Cursor(ptr)
    }

    /// Marks the sorted order as invalid (the heap invariant is still valid).
    ///
    /// Call this after a lazy key increase through [`Cursor::element_mut`].
    #[inline]
    pub fn mark_unsorted(&mut self) {
        self.is_sorted = false;
    }

    /// Records that the element behind `cursor` had an arbitrary key change
    /// (increase or decrease). O(1); the element is re-positioned lazily on the
    /// next [`front`](Self::front) / [`pop_front`](Self::pop_front) / sort, so the
    /// buffer layout does not move now.
    pub fn mark_changed(&mut self, cursor: Cursor<T>) {
        self.is_sorted = false;
        // SAFETY: cursor belongs to this heap (caller contract).
        let slot = unsafe { (*cursor.0.as_ptr()).heap_slot_mut() };
        if slot.pidx == NOT_PENDING {
            slot.pidx = self.pending.len() as u32;
            self.pending.push(cursor.0);
        }
    }

    /// Re-positions every pending element. Amortised O(p log n).
    fn apply_pending(&mut self) {
        while let Some(ptr) = self.pending.pop() {
            // SAFETY: pending only holds live elements of this heap; removed
            // elements are unlinked from `pending` in `remove_at`.
            let i = unsafe {
                let node = &mut *ptr.as_ptr();
                let key = node.heap_key();
                let slot = node.heap_slot_mut();
                slot.pidx = NOT_PENDING;
                slot.rec = key;
                slot.pos as usize
            };
            unsafe { self.fix(i) };
        }
    }

    /// Unlinks the element at buffer index `i` from `pending`, if it is there.
    #[inline]
    unsafe fn unlink_pending(&mut self, i: usize) {
        let pidx = unsafe { self.node(i).heap_slot().pidx };
        if pidx == NOT_PENDING {
            return;
        }
        self.pending.swap_remove(pidx as usize);
        if let Some(moved) = self.pending.get_mut(pidx as usize) {
            unsafe { moved.as_mut().heap_slot_mut().pidx = pidx };
        }
        unsafe { self.node_mut(i).heap_slot_mut().pidx = NOT_PENDING };
    }

    /// Sorts the backing buffer if it is not already sorted and then returns `true`.
    ///
    /// The order is by the live key, ties broken by insertion order.
    ///
    /// # Complexity
    /// O(n log n) when unsorted; O(1) when already sorted.
    #[inline]
    pub fn sort_by(&mut self) -> bool {
        if self.is_sorted {
            return false;
        }

        // SAFETY: every pointer in `self.data` is a live, heap-owned allocation.
        unsafe {
            for ptr in self.data.iter_mut() {
                let node = ptr.as_mut();
                let key = node.heap_key();
                let slot = node.heap_slot_mut();
                slot.rec = key;
                slot.pidx = NOT_PENDING;
            }

            self.pending.clear();

            self.data.sort_unstable_by(|a, b| {
                let (sa, sb) = (a.as_ref().heap_slot(), b.as_ref().heap_slot());
                (sa.rec, sa.seq).cmp(&(sb.rec, sb.seq))
            });

            for (i, ptr) in self.data.iter_mut().enumerate() {
                ptr.as_mut().heap_slot_mut().pos = i as u32;
            }
        }
        self.is_sorted = true;
        true
    }

    /// Returns a cursor to the smallest (front) element without removing it,
    /// or `None` if the heap is empty.
    ///
    /// # Complexity
    /// O(1) amortised.
    #[inline]
    pub fn front(&mut self) -> Option<Cursor<T>> {
        self.apply_pending();
        if self.repair_root() {
            self.is_sorted = false;
        }
        self.data.first().copied().map(Cursor)
    }

    /// Returns a cursor to the largest (back) element without removing it.
    ///
    /// Sorts the heap first if necessary (O(n log n)); rarely needed.
    #[inline]
    pub fn back(&mut self) -> Option<Cursor<T>> {
        self.sort_by();
        self.data.last().copied().map(Cursor)
    }

    /// Returns a cursor to the element at position `index`, or `None` if out
    /// of bounds.
    ///
    /// The index is only meaningful after the heap has been sorted — call
    /// [`sort_by`](Self::sort_by) first.
    #[inline]
    pub fn get(&self, index: usize) -> Option<Cursor<T>> {
        self.data.get(index).copied().map(Cursor)
    }

    /// Removes the element at buffer index `i` and returns it.
    unsafe fn remove_at(&mut self, i: usize) -> T {
        unsafe { self.unlink_pending(i) };
        let last = self.data.len() - 1;
        if i != last {
            unsafe { self.swap(i, last) };
        }
        // SAFETY: data is non-empty because `i` was a valid index.
        let ptr = unsafe { self.data.pop().unwrap_unchecked() };
        if i < self.data.len() {
            unsafe { self.fix(i) };
        }
        // Removing the last element of a sorted buffer keeps it sorted.
        if i != last {
            self.is_sorted = false;
        }
        // SAFETY: LazyHeap holds the sole Box for this pointer.
        *unsafe { Box::from_raw(ptr.as_ptr()) }
    }

    /// Removes and returns the smallest (front) element, or `None` if empty.
    ///
    /// # Complexity
    /// O(log n).
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        self.front()?;
        Some(unsafe { self.remove_at(0) })
    }

    /// Removes and returns the largest (back) element, or `None` if empty.
    ///
    /// Sorts the heap first if necessary.
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        self.sort_by();
        let last = self.data.len().checked_sub(1)?;
        Some(unsafe { self.remove_at(last) })
    }

    /// Removes and returns the element identified by `cursor`.
    ///
    /// # Complexity
    /// O(log n).
    pub fn remove(&mut self, cursor: Cursor<T>) -> T {
        debug_assert!(!self.data.is_empty());
        // SAFETY: cursor belongs to this heap (caller contract).
        let i = unsafe { cursor.element().heap_slot().pos as usize };
        debug_assert!(std::ptr::eq(self.data[i].as_ptr(), cursor.0.as_ptr()));
        unsafe { self.remove_at(i) }
    }

    /// Returns an iterator that yields a [`Cursor`] for each element in sorted
    /// order.
    ///
    /// Sorts the heap first if necessary. The returned [`RawIter`] holds raw
    /// pointers into the backing buffer; do not mutate or drop the heap while
    /// it is alive.
    #[inline]
    pub fn iter(&mut self) -> RawIter<T> {
        self.sort_by();
        RawIter {
            first: utils::RawSliceIter::new(self.data.as_slice()),
            second: utils::RawSliceIter::new(&[]),
        }
    }

    /// Removes all elements, dropping each one.
    #[inline]
    pub fn clear(&mut self) {
        self.pending.clear();
        while let Some(ptr) = self.data.pop() {
            // SAFETY: LazyHeap owns the sole Box for every pointer it stores.
            drop(unsafe { Box::from_raw(ptr.as_ptr()) });
        }
        self.is_sorted = true;
    }

    /// Shrinks the backing buffer's capacity as close to its current length
    /// as possible.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}

impl<T: HeapNode> Default for LazyHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: HeapNode> Drop for LazyHeap<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T: HeapNode>(&'a mut LazyHeap<T>);

        impl<'a, T: HeapNode> Drop for DropGuard<'a, T> {
            fn drop(&mut self) {
                // Only runs when a destructor panicked; keep freeing the rest.
                while let Some(ptr) = self.0.data.pop() {
                    drop(unsafe { Box::from_raw(ptr.as_ptr()) });
                }
            }
        }

        let guard = DropGuard(self);
        while let Some(ptr) = guard.0.data.pop() {
            drop(unsafe { Box::from_raw(ptr.as_ptr()) });
        }
        std::mem::forget(guard);
    }
}

/// A non-owning, pointer-sized handle to an element stored in a [`LazyHeap`].
///
/// Using a stale cursor is undefined behaviour.
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Cursor<T>(NonNull<T>);

impl<T> Clone for Cursor<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Cursor<T> {}

impl<T> Cursor<T> {
    /// Returns a shared reference to the value this cursor points to.
    ///
    /// # Safety
    /// The cursor must be valid (see the [type-level docs](Self)).
    #[inline]
    pub unsafe fn element(&self) -> &T {
        unsafe { self.0.as_ref() }
    }

    /// Returns a mutable reference to the value this cursor points to.
    ///
    /// The heap key may only be *increased* through this reference (followed by
    /// [`LazyHeap::mark_unsorted`]); any other key change must be followed by
    /// [`LazyHeap::mark_changed`].
    ///
    /// # Safety
    /// - The cursor must be valid (see the [type-level docs](Self)).
    /// - No other reference to the same element may be alive simultaneously.
    #[inline]
    pub unsafe fn element_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }

    /// Returns the raw pointer underlying this cursor.
    ///
    /// **Never** reconstruct a `Box` from this pointer — doing so transfers
    /// ownership out of the heap and causes a double-free.
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
}

/// Raw iterator over the heap buffer which doesn't carry a lifetime.
///
/// # Safety
/// You should track changes of the heap yourself.
pub struct RawIter<T> {
    first: utils::RawSliceIter<NonNull<T>>,
    second: utils::RawSliceIter<NonNull<T>>,
}

impl<T> Iterator for RawIter<T> {
    type Item = Cursor<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.first.next() {
            // SAFETY: `val` points into the heap buffer, which the caller keeps
            // alive and unmodified per this type's safety contract.
            Some(val) => Some(Cursor(unsafe { val.read() })),
            None => {
                std::mem::swap(&mut self.first, &mut self.second);
                self.first.next().map(|val| Cursor(unsafe { val.read() }))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.first.len() + self.second.len();
        (left, Some(left))
    }
}

impl<T> ExactSizeIterator for RawIter<T> {}

impl<T> Clone for RawIter<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone(),
        }
    }
}

unsafe impl<T: HeapNode + Send> Send for LazyHeap<T> {}
unsafe impl<T: HeapNode + Sync> Sync for LazyHeap<T> {}
unsafe impl<T: Send> Send for RawIter<T> {}
unsafe impl<T: Sync> Sync for RawIter<T> {}
unsafe impl<T: Send> Send for Cursor<T> {}
unsafe impl<T: Sync> Sync for Cursor<T> {}
