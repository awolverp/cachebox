use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;

/// [`LinkedList`]'s node
pub struct Node<T> {
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
    element: T,
}

impl<T> Node<T> {
    fn new(element: T) -> Self {
        Node {
            next: None,
            prev: None,
            element,
        }
    }

    #[allow(clippy::boxed_local)]
    fn into_element(self: Box<Self>) -> T {
        self.element
    }

    pub fn element(&self) -> &T {
        &self.element
    }
}

/// A doubly-linked list with owned nodes.
///
/// The `LinkedList` allows pushing and popping elements at either end
/// in constant time.
pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    _marker: PhantomData<Box<Node<T>>>,
}

// private methods
impl<T> LinkedList<T> {
    /// Adds the given node to the front of the list.
    ///
    /// # Safety
    /// `node` must point to a valid node that was boxed and leaked using the list's allocator.
    /// This method takes ownership of the node, so the pointer should not be used again.
    #[inline]
    unsafe fn push_front_node(&mut self, node: NonNull<Node<T>>) {
        // This method takes care not to create mutable references to whole nodes,
        // to maintain validity of aliasing pointers into `element`.
        unsafe {
            (*node.as_ptr()).next = self.head;
            (*node.as_ptr()).prev = None;
            let node = Some(node);

            match self.head {
                None => self.tail = node,
                // Not creating new mutable (unique!) references overlapping `element`.
                Some(head) => (*head.as_ptr()).prev = node,
            }

            self.head = node;
            self.len += 1;
        }
    }

    /// Removes and returns the node at the front of the list.
    #[inline]
    fn pop_front_node(&mut self) -> Option<Box<Node<T>>> {
        // This method takes care not to create mutable references to whole nodes,
        // to maintain validity of aliasing pointers into `element`.
        self.head.map(|node| unsafe {
            let node = Box::from_raw(node.as_ptr());
            self.head = node.next;

            match self.head {
                None => self.tail = None,
                // Not creating new mutable (unique!) references overlapping `element`.
                Some(head) => (*head.as_ptr()).prev = None,
            }

            self.len -= 1;
            node
        })
    }

    /// Adds the given node to the back of the list.
    ///
    /// # Safety
    /// `node` must point to a valid node that was boxed and leaked using the list's allocator.
    /// This method takes ownership of the node, so the pointer should not be used again.
    #[inline]
    unsafe fn push_back_node(&mut self, node: NonNull<Node<T>>) {
        // This method takes care not to create mutable references to whole nodes,
        // to maintain validity of aliasing pointers into `element`.
        unsafe {
            (*node.as_ptr()).next = None;
            (*node.as_ptr()).prev = self.tail;
            let node = Some(node);

            match self.tail {
                None => self.head = node,
                // Not creating new mutable (unique!) references overlapping `element`.
                Some(tail) => (*tail.as_ptr()).next = node,
            }

            self.tail = node;
            self.len += 1;
        }
    }

    /// Removes and returns the node at the back of the list.
    #[inline]
    fn pop_back_node(&mut self) -> Option<Box<Node<T>>> {
        // This method takes care not to create mutable references to whole nodes,
        // to maintain validity of aliasing pointers into `element`.
        self.tail.map(|node| unsafe {
            let node = Box::from_raw(node.as_ptr());
            self.tail = node.prev;

            match self.tail {
                None => self.head = None,
                // Not creating new mutable (unique!) references overlapping `element`.
                Some(tail) => (*tail.as_ptr()).next = None,
            }

            self.len -= 1;
            node
        })
    }

    /// Unlinks the specified node from the current list.
    ///
    /// Warning: this will not check that the provided node belongs to the current list.
    ///
    /// This method takes care not to create mutable references to `element`, to
    /// maintain validity of aliasing pointers.
    #[inline]
    unsafe fn unlink_node(&mut self, mut node: NonNull<Node<T>>) {
        let node = unsafe { node.as_mut() }; // this one is ours now, we can create an &mut.

        // Not creating new mutable (unique!) references overlapping `element`.
        match node.prev {
            Some(prev) => unsafe { (*prev.as_ptr()).next = node.next },
            // this node is the head node
            None => self.head = node.next,
        };

        match node.next {
            Some(next) => unsafe { (*next.as_ptr()).prev = node.prev },
            // this node is the tail node
            None => self.tail = node.prev,
        };

        self.len -= 1;
    }

    /// Unlinks the specified node from the current list and returns the item.
    ///
    /// # Safety
    /// This will not check that the provided node belongs to the current list.
    unsafe fn remove_node(&mut self, node: NonNull<Node<T>>) -> T {
        unsafe {
            self.unlink_node(node);
            let node = Box::from_raw(node.as_ptr());
            node.element
        }
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
    pub const fn new() -> Self {
        LinkedList {
            head: None,
            tail: None,
            len: 0,
            _marker: PhantomData,
        }
    }

    /// Returns `true` if the `LinkedList` is empty.
    ///
    /// This operation should compute in *O*(1) time.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Returns the length of the `LinkedList`.
    ///
    /// This operation should compute in *O*(1) time.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Removes all elements from the `LinkedList`.
    ///
    /// This operation should compute in *O*(*n*) time.
    #[inline]
    pub fn clear(&mut self) {
        drop(LinkedList {
            head: self.head.take(),
            tail: self.tail.take(),
            len: mem::take(&mut self.len),
            _marker: PhantomData,
        });
    }

    /// Returns a [`Cursor`] to the front node, or `None` if the list is empty.
    #[inline]
    #[must_use]
    pub fn cursor_front(&self) -> Option<Cursor<T>> {
        self.head.map(Cursor::new)
    }

    /// Returns a [`Cursor`] to the back node, or `None` if the list is empty.
    #[inline]
    #[must_use]
    pub fn cursor_back(&self) -> Option<Cursor<T>> {
        self.tail.map(Cursor::new)
    }

    /// Adds an element to the front of the list and returns a [`Cursor`] to it.
    ///
    /// This operation should compute in *O*(1) time.
    #[inline]
    pub fn push_front(&mut self, elt: T) -> Cursor<T> {
        let node = Box::new(Node::new(elt));
        let node_ptr = NonNull::from(Box::leak(node));

        // SAFETY: node_ptr is a unique pointer to a node we boxed with self.alloc and leaked
        unsafe {
            self.push_front_node(node_ptr);
        }
        Cursor::new(node_ptr)
    }

    /// Removes the first element and returns it, or `None` if the list is
    /// empty.
    ///
    /// This operation should compute in *O*(1) time.
    #[inline]
    pub fn pop_front(&mut self) -> Option<T> {
        self.pop_front_node().map(Node::into_element)
    }

    /// Adds an element to the back of the list and returns a [`Cursor`] to it.
    ///
    /// This operation should compute in *O*(1) time.
    #[inline]
    pub fn push_back(&mut self, elt: T) -> Cursor<T> {
        let node = Box::new(Node::new(elt));
        let node_ptr = NonNull::from(Box::leak(node));

        // SAFETY: node_ptr is a unique pointer to a node we boxed with self.alloc and leaked
        unsafe {
            self.push_back_node(node_ptr);
        }
        Cursor::new(node_ptr)
    }

    /// Removes the last element from a list and returns it, or `None` if
    /// it is empty.
    ///
    /// This operation should compute in *O*(1) time.
    #[inline]
    pub fn pop_back(&mut self) -> Option<T> {
        self.pop_back_node().map(Node::into_element)
    }

    /// Returns a raw, lifetime-free iterator over the nodes of a LinkedList.
    ///
    /// # Safety
    /// The iterator must not outlive the list it was created from, and the list must not be structurally modified.
    pub unsafe fn iter(&self) -> RawIter<T> {
        RawIter {
            head: self.head,
            len: self.len,
        }
    }
}

unsafe impl<#[may_dangle] T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T>(&'a mut LinkedList<T>);

        impl<'a, T> Drop for DropGuard<'a, T> {
            fn drop(&mut self) {
                // Continue the same loop we do below. This only runs when a destructor has
                // panicked. If another one panics this will abort.
                while self.0.pop_front_node().is_some() {}
            }
        }

        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard(self);
        while guard.0.pop_front_node().is_some() {}
        mem::forget(guard);
    }
}

/// An opaque handle to a node in a [`LinkedList`].
///
/// Obtained via [`LinkedList::push_front`], [`LinkedList::push_back`],
/// [`LinkedList::cursor_front`], or [`LinkedList::cursor_back`].
///
/// `Cursor` is `Copy`; cloning or copying it produces a second handle to the
/// *same* node.  Two cursors compare equal iff they point at the same node.
///
/// # Safety invariant
/// Every `unsafe` method on `Cursor` requires that:
/// - the cursor was obtained from the list it is passed to, **and**
/// - the node has not yet been removed from that list.
///
/// Violating either condition is undefined behaviour.
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
    #[inline]
    fn new(node: NonNull<Node<T>>) -> Self {
        Cursor(node)
    }

    /// Returns a shared reference to the element this cursor points to.
    ///
    /// # Safety
    /// See the [struct-level safety invariant](Cursor).
    /// The returned reference borrows for `'a`, which the caller must
    /// ensure does not outlive the node or the list.
    #[inline]
    pub unsafe fn element<'a>(&self) -> &'a T {
        &(*self.0.as_ptr()).element
    }

    /// Returns a mutable reference to the element this cursor points to.
    ///
    /// # Safety
    /// See the [struct-level safety invariant](Cursor).
    /// In addition, no other reference to this element may exist for the
    /// duration of the returned `'a` borrow.
    #[inline]
    pub unsafe fn element_mut<'a>(&mut self) -> &'a mut T {
        &mut (*self.0.as_ptr()).element
    }

    /// Moves this node to the front of `list`.
    ///
    /// # Safety
    /// See the [struct-level safety invariant](Cursor).
    #[inline]
    pub unsafe fn move_to_front(self, list: &mut LinkedList<T>) {
        list.unlink_node(self.0);
        list.push_front_node(self.0);
    }

    /// Moves this node to the back of `list`.
    ///
    /// # Safety
    /// See the [struct-level safety invariant](Cursor).
    #[inline]
    pub unsafe fn move_to_back(self, list: &mut LinkedList<T>) {
        list.unlink_node(self.0);
        list.push_back_node(self.0);
    }

    /// Unlinks this node from `list` and returns its element.
    ///
    /// Consumes the cursor so it cannot be used after removal.
    ///
    /// # Safety
    /// See the [struct-level safety invariant](Cursor).
    #[inline]
    pub unsafe fn unlink(self, list: &mut LinkedList<T>) -> T {
        list.remove_node(self.0)
    }
}

/// A raw, lifetime-free iterator over the nodes of a [`LinkedList`].
///
/// Yields a [`Cursor`] for each node, from front to back.
///
/// Obtained via [`LinkedList::iter`].
///
/// # Safety invariant
/// The iterator must not outlive the list it was created from, and the list
/// must not be structurally modified (nodes added or removed) while iterating.
/// Violating either condition is undefined behaviour.
pub struct RawIter<T> {
    head: Option<NonNull<Node<T>>>,
    len: usize,
}

impl<T> Iterator for RawIter<T> {
    type Item = Cursor<T>;

    #[inline]
    fn next(&mut self) -> Option<Cursor<T>> {
        if self.len == 0 {
            return None;
        }
        self.head.map(|node| {
            self.len -= 1;
            // SAFETY: node is a valid, live pointer for as long as the list lives.
            self.head = unsafe { (*node.as_ptr()).next };
            Cursor::new(node)
        })
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
