use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::handle_alloc_error;
use std::alloc::Layout;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;
use std::ptr::{self};

/// Maximum number of nodes parked on the free list
const FREE_LIST_CAPACITY: usize = 256;

/// Intrusive doubly-linked pointers shared by every node and the sentinel.
pub struct Links {
    prev: *mut Links,
    next: *mut Links,
}

impl Links {
    /// Returns a `Links` with both pointers null.
    #[inline]
    const fn empty() -> Self {
        Links {
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }
}

/// A single list element: link pointers plus the stored value.
///
/// `#[repr(C)]` guarantees `links` is the first field at offset 0, so a
/// `*mut Links` obtained from traversing the list can always be reinterpreted
/// as `*mut Node<T>` (and vice versa) - this is what lets `Cursor<T>` and the
/// sentinel-based traversal in `push_front_node`/`unlink_node` operate on
/// plain `Links` pointers without knowing `T`.
#[repr(C)]
pub struct Node<T> {
    links: Links,
    element: T,
}

/// A doubly-linked list with an internal, bounded free-list for node reuse.
pub struct LinkedList<T> {
    sentinel: NonNull<Links>,
    free_head: *mut Links,
    free_len: usize,
    len: usize,
    _marker: PhantomData<Box<Node<T>>>,
}

impl<T> LinkedList<T> {
    /// Adds `node` to the front of the list.
    ///
    /// # Safety
    ///
    /// - `node` must point to a valid, currently unlinked `Links` belonging
    ///   to this list's allocations (freshly allocated via `alloc_node`/`get_node`,
    ///   or previously unlinked via `unlink_node`/`pop_*_node`).
    ///
    /// - The caller must not use `node` again as an "unlinked" node until it is
    ///   unlinked again.
    #[inline]
    unsafe fn push_front_node(&mut self, node: NonNull<Links>) {
        let s = self.sentinel.as_ptr();
        let n = node.as_ptr();
        unsafe {
            // SAFETY: sentinel always valid; `n` valid per fn contract.
            let first = (*s).next;
            (*n).prev = s;
            (*n).next = first;
            (*first).prev = n;
            (*s).next = n;
        }
    }

    /// Adds `node` to the back of the list.
    ///
    /// # Safety
    ///
    /// Same contract as [`push_front_node`](Self::push_front_node).
    #[inline]
    unsafe fn push_back_node(&mut self, node: NonNull<Links>) {
        let s = self.sentinel.as_ptr();
        let n = node.as_ptr();
        unsafe {
            // SAFETY: sentinel always valid; `n` valid per fn contract.
            let last = (*s).prev;
            (*n).next = s;
            (*n).prev = last;
            (*last).next = n;
            (*s).prev = n;
        }
    }

    /// Removes and returns the node at the front of the list, if any.
    #[inline]
    fn pop_front_node(&mut self) -> Option<NonNull<Links>> {
        if self.len == 0 {
            return None;
        }
        let s = self.sentinel.as_ptr();
        let node = unsafe {
            // SAFETY: `len != 0` -> `(*s).next` is a real node.
            let node = (*s).next;
            let second = (*node).next;
            (*second).prev = s;
            (*s).next = second;
            node
        };
        self.len -= 1;
        // SAFETY: read from a valid node above, so non-null.
        Some(unsafe { NonNull::new_unchecked(node) })
    }

    /// Removes and returns the node at the back of the list, if any.
    #[inline]
    fn pop_back_node(&mut self) -> Option<NonNull<Links>> {
        if self.len == 0 {
            return None;
        }
        let s = self.sentinel.as_ptr();
        let node = unsafe {
            // SAFETY: `len != 0` -> `(*s).prev` is a real node.
            let node = (*s).prev;
            let before = (*node).prev;
            (*before).next = s;
            (*s).prev = before;
            node
        };
        self.len -= 1;
        // SAFETY: read from a valid node above, so non-null.
        Some(unsafe { NonNull::new_unchecked(node) })
    }

    /// Unlinks `node` from the list, without deallocating or reading its element.
    ///
    /// # Safety
    ///
    /// `node` must point to a node currently linked into this list (not the sentinel).
    #[inline]
    unsafe fn unlink_node(&mut self, node: NonNull<Links>) {
        let n = node.as_ptr();
        unsafe {
            // SAFETY: `n` linked per fn contract; `prev`/`next` valid.
            let prev = (*n).prev;
            let next = (*n).next;
            (*prev).next = next;
            (*next).prev = prev;
        }
    }

    /// Pushes `node` onto the internal free list for reuse, or deallocates
    /// it when the free list is already at [`FREE_LIST_CAPACITY`] entries.
    ///
    /// # Safety
    ///
    /// `node` must be unlinked and its `element` must already be logically
    /// moved out (dropped or read via `ptr::read`) — this only reuses the
    /// `Links`/allocation, not the `T` storage.
    #[inline]
    unsafe fn recycle_node(&mut self, node: *mut Links) {
        if self.free_len >= FREE_LIST_CAPACITY {
            // SAFETY: unlinked + moved-out per fn contract; allocated with
            // this exact layout in `alloc_node`.
            unsafe { dealloc(node as *mut u8, Layout::new::<Node<T>>()) };
            return;
        }
        let free_head = self.free_head;
        unsafe {
            // SAFETY: valid per fn contract; `next` write doesn't touch `element`.
            (*node).next = free_head;
        }
        self.free_head = node;
        self.free_len += 1;
    }

    /// Reads the element out of `node` and recycles the node's storage.
    ///
    /// # Safety
    ///
    /// `node` must point to a node whose `element` is initialized and which is
    /// no longer linked into the list (already unlinked by the caller).
    #[inline]
    unsafe fn take_element_and_recycle(&mut self, node: NonNull<Links>) -> T {
        let node_ptr = node.as_ptr() as *mut Node<T>;
        // SAFETY: `element` initialized per fn contract; `ptr::read` doesn't drop.
        let element = unsafe { ptr::read(&(*node_ptr).element) };
        // SAFETY: element moved out above, node now safe to recycle.
        unsafe { self.recycle_node(node.as_ptr()) };
        element
    }

    /// Unlinks `node` from the list and returns its element.
    ///
    /// # Safety
    ///
    /// `node` must point to a node currently linked into this list
    #[inline]
    unsafe fn remove_node(&mut self, node: NonNull<Links>) -> T {
        // SAFETY: caller guarantees `node` is linked.
        unsafe { self.unlink_node(node) };
        self.len -= 1;
        // SAFETY: just unlinked above; `element` still initialized.
        unsafe { self.take_element_and_recycle(node) }
    }

    /// Drops every linked element and parks its node on the free list.
    ///
    /// Elements whose destructors panic are handled by the caller's
    /// drop-guard strategy; nodes are recycled even on the panic path.
    fn drop_nodes(&mut self) {
        let s = self.sentinel.as_ptr();
        while self.len != 0 {
            unsafe {
                // SAFETY: `self.len != 0`, so `(*s).next` points to a real node.
                let node = (*s).next;
                let second = (*node).next;
                (*second).prev = s;
                (*s).next = second;

                let node_ptr = node as *mut Node<T>;
                // SAFETY: layout-compatible (see `Node` docs); `element` still init.
                ptr::drop_in_place(&mut (*node_ptr).element);
                // SAFETY: element dropped above; node already unlinked.
                self.recycle_node(node);
            }
            self.len -= 1;
        }
    }

    /// Frees every entry on the internal free list.
    ///
    /// # Safety
    ///
    /// Must only be called when no other references into the free list exist
    /// (e.g. from `Drop`); each recycled node must have been allocated with
    /// `Layout::new::<Node<T>>()`.
    unsafe fn drop_freelist(&mut self) {
        let mut node = self.free_head;
        while !node.is_null() {
            // SAFETY: non-null, pushed by `recycle_node` -> valid `Node<T>` alloc.
            let next = unsafe { (*node).next };
            // SAFETY: allocated with this exact layout; not reused after this.
            unsafe { dealloc(node as *mut u8, Layout::new::<Node<T>>()) };
            node = next;
        }
        self.free_head = ptr::null_mut();
        self.free_len = 0;
    }

    /// Returns a node holding `elt`: reuses a free-list node when one is
    /// available, otherwise allocates a fresh one.
    #[inline]
    fn get_node(&mut self, elt: T) -> *mut Node<T> {
        if !self.free_head.is_null() {
            let links = self.free_head;
            // SAFETY: non-null, threaded via `recycle_node`.
            self.free_head = unsafe { (*links).next };
            self.free_len -= 1;
            let node = links as *mut Node<T>;
            // SAFETY: layout-compatible (see `Node` docs); only `element` is stale.
            unsafe { ptr::write(&mut (*node).element, elt) };
            node
        } else {
            Self::alloc_node(elt)
        }
    }

    /// Cold fallback of `get_node`: allocates a brand-new node.
    #[inline(never)]
    #[cold]
    fn alloc_node(elt: T) -> *mut Node<T> {
        Box::into_raw(Box::new(Node {
            links: Links::empty(),
            element: elt,
        }))
    }
}

impl<T> Default for LinkedList<T> {
    /// Creates an empty `LinkedList<T>`.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LinkedList<T> {
    /// Creates an empty `LinkedList`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let layout = Layout::new::<Links>();
        let raw = unsafe {
            let p = alloc(layout) as *mut Links;
            if p.is_null() {
                handle_alloc_error(layout);
            }
            // SAFETY: `p` allocated with `layout`, checked non-null above.
            ptr::write(p, Links { prev: p, next: p });
            p
        };
        // SAFETY: non-null checked above (or diverged).
        let sentinel = unsafe { NonNull::new_unchecked(raw) };
        LinkedList {
            sentinel,
            free_head: ptr::null_mut(),
            free_len: 0,
            len: 0,
            _marker: PhantomData,
        }
    }

    /// Returns `true` if the list contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the list.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the number of elements in the list.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.len + self.free_len
    }

    /// Removes all elements, dropping each element's value.
    #[inline]
    pub fn clear(&mut self) {
        self.drop_nodes();
    }

    /// Returns a cursor to the front element, or `None` if the list is empty.
    #[inline]
    #[must_use]
    pub fn cursor_front(&self) -> Option<Cursor<T>> {
        if self.len == 0 {
            return None;
        }
        let s = self.sentinel.as_ptr();
        // SAFETY: sentinel always valid; `len != 0` -> real node, not `s`.
        let node = unsafe { (*s).next };
        // SAFETY: non-null above; layout-compatible (see `Node` docs).
        Some(Cursor(unsafe {
            NonNull::new_unchecked(node as *mut Node<T>)
        }))
    }

    /// Returns a cursor to the back element, or `None` if the list is empty.
    #[inline]
    #[must_use]
    pub fn cursor_back(&self) -> Option<Cursor<T>> {
        if self.len == 0 {
            return None;
        }
        let s = self.sentinel.as_ptr();
        // SAFETY: sentinel always valid; `len != 0` -> real node.
        let node = unsafe { (*s).prev };
        // SAFETY: same as `cursor_front`.
        Some(Cursor(unsafe {
            NonNull::new_unchecked(node as *mut Node<T>)
        }))
    }

    /// Inserts `elt` at the front of the list and returns a cursor to it.
    #[inline]
    pub fn push_front(&mut self, elt: T) -> Cursor<T> {
        let node = self.get_node(elt);
        // SAFETY: `get_node` never returns null.
        let links = unsafe { NonNull::new_unchecked(node as *mut Links) };

        // SAFETY: freshly obtained node, unlinked (see `push_front_node` contract).
        unsafe { self.push_front_node(links) };
        self.len += 1;
        // SAFETY: same pointer validated above.
        Cursor(unsafe { NonNull::new_unchecked(node) })
    }

    /// Removes and returns the front element, or `None` if the list is empty.
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        let node = self.pop_front_node()?;
        // SAFETY: `node` was just unlinked by `pop_front_node`, so its
        // `element` is still initialized and it is safe to take/recycle.
        let element = unsafe { self.take_element_and_recycle(node) };
        Some(element)
    }

    /// Inserts `elt` at the back of the list and returns a cursor to it.
    #[inline]
    pub fn push_back(&mut self, elt: T) -> Cursor<T> {
        let node = self.get_node(elt);
        // SAFETY: `get_node` always returns a non-null, freshly-usable `Node<T>` pointer.
        let links = unsafe { NonNull::new_unchecked(node as *mut Links) };

        // SAFETY: `links` refers to a node that was just obtained and is
        // not linked into any list yet.
        unsafe { self.push_back_node(links) };
        self.len += 1;
        // SAFETY: `node` is the same non-null pointer validated above.
        Cursor(unsafe { NonNull::new_unchecked(node) })
    }

    /// Removes and returns the back element, or `None` if the list is empty.
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        let node = self.pop_back_node()?;
        // SAFETY: `node` was just unlinked by `pop_back_node`, so its
        // `element` is still initialized and it is safe to take/recycle.
        let element = unsafe { self.take_element_and_recycle(node) };
        Some(element)
    }

    /// Returns a raw, unsynchronized iterator over cursors into the list.
    ///
    /// # Safety
    ///
    /// The caller must not mutate or drop the list while the returned
    /// `RawIter` (or any `Cursor` obtained from it) is in use, and must not
    /// call [`Cursor::unlink`], [`Cursor::move_to_front`], or
    /// [`Cursor::move_to_back`] on a yielded cursor while iteration is still
    /// in progress, since that would invalidate `RawIter::next`.
    #[inline]
    pub unsafe fn iter(&self) -> RawIter<T> {
        let s = self.sentinel.as_ptr();
        // SAFETY: sentinel is always valid, regardless of `self.len`.
        let next = unsafe { (*s).next };
        RawIter {
            next,
            len: self.len,
            _marker: PhantomData,
        }
    }
}

// Guard ensures that if dropping an element panics, we still free
// the free-list allocations instead of leaking them (the sentinel
// is freed unconditionally below regardless of panics).
struct DropGuard<'a, T>(&'a mut LinkedList<T>);

impl<'a, T> Drop for DropGuard<'a, T> {
    fn drop(&mut self) {
        self.0.drop_nodes();
        // SAFETY: `self` is being torn down during unwind; no other
        // references to the free list exist, satisfying `drop_freelist`'s contract.
        unsafe { self.0.drop_freelist() };
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let guard = DropGuard(self);
        guard.0.drop_nodes();
        // SAFETY: `drop_nodes` completed without panicking; the free list is
        // only touched here and then never again (guard is forgotten next).
        unsafe { guard.0.drop_freelist() };
        mem::forget(guard);
        // SAFETY: `self.sentinel` was allocated in `new` with exactly this
        // layout (`Layout::new::<Links>()`), and after this point `self` is
        // being destroyed, so the pointer is never dereferenced again.
        unsafe {
            dealloc(self.sentinel.as_ptr() as *mut u8, Layout::new::<Links>());
        }
    }
}

/// A handle to a single node in a `LinkedList`.
///
/// `Cursor` is a thin, `Copy`able pointer to a node, obtained from
/// [`LinkedList::push_front`], [`LinkedList::push_back`],
/// [`LinkedList::cursor_front`], or [`LinkedList::cursor_back`], and used to
/// later access or reposition that node via [`Cursor::element`],
/// [`Cursor::move_to_front`], [`Cursor::move_to_back`], or [`Cursor::unlink`].
///
/// `#[repr(transparent)]` over `NonNull<Node<T>>` means a `Cursor<T>` has the
/// exact same layout as the raw pointer it wraps — no extra state is tracked,
/// so the cursor does *not* know which `LinkedList` it came from, whether the
/// node is still linked, or whether other `Cursor`s alias the same node. All
/// of that is the caller's responsibility, which is why every non-trivial
/// method on `Cursor` is `unsafe`.
///
/// Because it is just a pointer, `Cursor<T>` is cheaply `Copy`/`Clone`, and
/// equality (`PartialEq`/`Eq`) compares the underlying pointer, i.e. identity
/// of the node, not the value of the element.
#[repr(transparent)]
pub struct Cursor<T>(NonNull<Node<T>>);

// `NonNull<Node<T>>` is just a pointer; copying it is always safe.
impl<T> Clone for Cursor<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Cursor<T> {}

// Pointer equality: two cursors are equal if they point at the same node.
impl<T> PartialEq for Cursor<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Cursor<T> {}

impl<T> Cursor<T> {
    /// Returns the underlying node as a `Links` pointer, for use with the
    /// list's internal `Links`-based operations.
    ///
    /// Relies on `#[repr(C)]` on `Node<T>` placing `links` at offset 0, so the
    /// cast is always valid regardless of `T`.
    #[inline]
    fn links(&self) -> NonNull<Links> {
        let ptr = self.0.as_ptr() as *mut Links;
        // SAFETY: `self.0` is `NonNull`, so the reinterpreted pointer is non-null too.
        unsafe { NonNull::new_unchecked(ptr) }
    }

    /// Returns a reference to the node's element, with a caller-chosen lifetime.
    ///
    /// # Safety
    ///
    /// - The node this cursor points to must still be linked (or otherwise
    ///   kept alive) in some `LinkedList<T>`, i.e. not yet unlinked/recycled.
    ///
    /// - The returned `&'a T` must not outlive the underlying allocation, and
    ///   no `&mut T`/`element_mut` alias to the same node may exist while
    ///   this reference is live.
    #[inline]
    pub unsafe fn element<'a>(&self) -> &'a T {
        let node = self.0.as_ptr();
        // SAFETY: caller guarantees the node is still allocated/linked and
        // that no conflicting `&mut` aliases the element.
        unsafe { &(*node).element }
    }

    /// Returns a mutable reference to the node's element, with a caller-chosen lifetime.
    ///
    /// # Safety
    ///
    /// Same contract as [`element`](Self::element), plus: no other reference
    /// (shared or mutable) to this node's element may be alive at the same time.
    #[inline]
    pub unsafe fn element_mut<'a>(&mut self) -> &'a mut T {
        let node = self.0.as_ptr();
        // SAFETY: caller guarantees exclusive access to this node's element.
        unsafe { &mut (*node).element }
    }

    /// Moves the node this cursor points to the front of `list`.
    ///
    /// # Safety
    ///
    /// - The node must currently be linked into `list` (not some other list,
    ///   and not already unlinked/recycled).
    ///
    /// - No other `Cursor`/reference into this node may be used concurrently
    ///   with this call.
    #[inline]
    pub unsafe fn move_to_front(self, list: &mut LinkedList<T>) {
        let links = self.links();
        // SAFETY: caller guarantees `links` is currently linked into `list`.
        unsafe { list.unlink_node(links) };
        // SAFETY: `links` was just unlinked above, so it's safe to relink.
        unsafe { list.push_front_node(links) };
    }

    /// Moves the node this cursor points to the back of `list`.
    ///
    /// # Safety
    ///
    /// Same contract as [`move_to_front`](Self::move_to_front).
    #[inline]
    pub unsafe fn move_to_back(self, list: &mut LinkedList<T>) {
        let links = self.links();
        // SAFETY: caller guarantees `links` is currently linked into `list`.
        unsafe { list.unlink_node(links) };
        // SAFETY: `links` was just unlinked above, so it's safe to relink.
        unsafe { list.push_back_node(links) };
    }

    /// Removes the node this cursor points to from `list` and returns its element.
    ///
    /// # Safety
    ///
    /// - The node must currently be linked into `list`.
    ///
    /// - This consumes the cursor (`self`, by value) because the node is
    ///   deallocated/recycled afterward — the cursor must not be used again.
    #[inline]
    pub unsafe fn unlink(self, list: &mut LinkedList<T>) -> T {
        let links = self.links();
        // SAFETY: caller guarantees `links` is currently linked into `list`.
        unsafe { list.remove_node(links) }
    }
}

/// A raw, unsynchronized iterator over `Cursor`s in a `LinkedList`.
///
/// Created only via [`LinkedList::iter`], which is itself `unsafe` — see its
/// `# Safety` section for the invariants that make walking `next`/`len` here
/// sound (the list must not be mutated or dropped while this iterator, or any
/// `Cursor` it yields, is in use).
pub struct RawIter<T> {
    next: *mut Links,
    len: usize,
    _marker: PhantomData<NonNull<Node<T>>>,
}

impl<T> Iterator for RawIter<T> {
    type Item = Cursor<T>;

    #[inline]
    fn next(&mut self) -> Option<Cursor<T>> {
        if self.len == 0 {
            return None;
        }
        let node = self.next;
        self.len -= 1;
        // SAFETY: `self.len != 0` (checked above) guarantees `node` is a
        // currently-valid, linked node, so `(*node).next` is a valid read;
        // per `LinkedList::iter`'s contract, the list is not mutated/dropped
        // while this iterator is alive.
        self.next = unsafe { (*node).next };
        // SAFETY: `node` is non-null (came from a valid linked `Links`), and
        // by `#[repr(C)]` on `Node<T>` a `*mut Links` is a valid `*mut Node<T>`.
        let cursor = Cursor(unsafe { NonNull::new_unchecked(node as *mut Node<T>) });
        Some(cursor)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

unsafe impl<T: Send + Send> Send for LinkedList<T> {}
unsafe impl<T: Sync + Sync> Sync for LinkedList<T> {}
unsafe impl<T: Send + Send> Send for RawIter<T> {}
unsafe impl<T: Sync + Sync> Sync for RawIter<T> {}
unsafe impl<T: Send + Send> Send for Cursor<T> {}
unsafe impl<T: Sync + Sync> Sync for Cursor<T> {}
