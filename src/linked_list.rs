use crate::common::PreHashObject;
use std::ptr::NonNull;

/// A doubly-linked list implementation with optional head and tail nodes.
///
/// This list maintains references to the first and last nodes, and tracks the total number of elements.
/// Uses `NonNull` pointers for efficient memory management and allows for constant-time
/// insertion and deletion at both ends of the list.
pub struct LinkedList {
    pub head: Option<NonNull<Node>>, // front
    pub tail: Option<NonNull<Node>>, // back
    len: usize,
}

/// A node in a doubly-linked list, containing a reference to the previous and next nodes,
/// and storing a key-value pair as its element.
///
/// The node uses `NonNull` pointers for efficient memory management and allows for
/// constant-time insertion and deletion in the linked list.
pub struct Node {
    pub prev: Option<NonNull<Node>>,
    pub next: Option<NonNull<Node>>,
    pub element: (PreHashObject, pyo3::PyObject),
}

impl LinkedList {
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
            len: 0,
        }
    }

    #[inline]
    pub fn push_back(&mut self, key: PreHashObject, val: pyo3::PyObject) -> NonNull<Node> {
        unsafe {
            let node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                prev: None,
                next: None,
                element: (key, val),
            })));

            if let Some(old) = self.tail {
                (*old.as_ptr()).next = Some(node);
                (*node.as_ptr()).prev = Some(old);
            } else {
                // means list is empty, so this node is also can be the front of list
                debug_assert!(self.head.is_none(), "head is not None");
                self.head = Some(node);
            }

            self.tail = Some(node);
            self.len += 1;
            node
        }
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<(PreHashObject, pyo3::PyObject)> {
        unsafe {
            self.head.map(|node| {
                let boxed_node = Box::from_raw(node.as_ptr());
                debug_assert!(boxed_node.prev.is_none(), "head.prev is not None");

                self.head = boxed_node.next;

                match self.head {
                    None => self.tail = None,
                    // Not creating new mutable (unique!) references overlapping `element`.
                    Some(head) => (*head.as_ptr()).prev = None,
                }

                debug_assert!(self.len > 0, "self.len is zero");
                self.len -= 1;
                boxed_node.element
            })
        }
    }

    pub fn clear(&mut self) {
        while self.pop_front().is_some() {}
    }

    #[inline]
    pub unsafe fn remove(&mut self, node: NonNull<Node>) -> (PreHashObject, pyo3::PyObject) {
        let node = Box::from_raw(node.as_ptr());
        let result = node.element;

        match node.next {
            Some(next) => (*next.as_ptr()).prev = node.prev,
            None => {
                // Means this node is our self.tail
                self.tail = node.prev;
            }
        }

        match node.prev {
            Some(prev) => (*prev.as_ptr()).next = node.next,
            None => {
                // Means this node is our self.head
                self.head = node.next;
            }
        }

        self.len -= 1;
        result
    }

    #[inline]
    pub unsafe fn move_back(&mut self, node: NonNull<Node>) {
        if (*node.as_ptr()).next.is_none() {
            // Means this node is our self.tail
            return;
        }

        // unlink
        match (*node.as_ptr()).next {
            Some(next) => (*next.as_ptr()).prev = (*node.as_ptr()).prev,
            None => std::hint::unreachable_unchecked(),
        }

        match (*node.as_ptr()).prev {
            Some(prev) => (*prev.as_ptr()).next = (*node.as_ptr()).next,
            None => {
                // Means this node is our self.head
                self.head = (*node.as_ptr()).next;
            }
        }

        (*node.as_ptr()).next = None;
        (*node.as_ptr()).prev = None;

        // push_back again
        if let Some(old) = self.tail {
            (*old.as_ptr()).next = Some(node);
            (*node.as_ptr()).prev = Some(old);
        } else {
            // means list is empty, so this node is also can be the front of list
            debug_assert!(self.head.is_none(), "head is not None");
            self.head = Some(node);
        }

        self.tail = Some(node);
    }

    pub fn iter(&self) -> Iter {
        Iter {
            head: self.head,
            len: self.len,
        }
    }
}

pub struct Iter {
    head: Option<NonNull<Node>>,
    len: usize,
}

impl Iterator for Iter {
    type Item = NonNull<Node>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            None
        } else {
            self.head.inspect(|node| unsafe {
                self.len -= 1;
                self.head = (*node.as_ptr()).next;
            })
        }
    }
}

impl Drop for LinkedList {
    fn drop(&mut self) {
        struct DropGuard<'a>(&'a mut LinkedList);

        impl Drop for DropGuard<'_> {
            fn drop(&mut self) {
                // Continue the same loop we do below. This only runs when a destructor has
                // panicked. If another one panics this will abort.
                while self.0.pop_front().is_some() {}
            }
        }

        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard(self);
        while guard.0.pop_front().is_some() {}
        core::mem::forget(guard);
    }
}

unsafe impl Sync for Iter {}
unsafe impl Send for Iter {}
