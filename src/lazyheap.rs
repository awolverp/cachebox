use crate::common::NoLifetimeSliceIter;
use std::ptr::NonNull;

/// A heap data structure that lazily maintains sorting order.
///
/// `LazyHeap` allows for efficient insertion of elements without immediately sorting,
/// with the ability to defer sorting until necessary. This can improve performance
/// in scenarios where sorting is not immediately required.
///
/// ```
/// let mut heap = LazyHeap::new();
/// heap.push(5);
/// ```
pub struct LazyHeap<T> {
    data: std::collections::VecDeque<NonNull<T>>,
    is_sorted: bool,
}

/// An iterator for traversing elements in a `LazyHeap`.
///
/// This iterator allows sequential access to the elements of a `LazyHeap`,
/// maintaining the current position and total length during iteration.
///
/// # Safety
///
/// This iterator uses raw pointers and requires careful management to ensure
/// memory safety and prevent use-after-free or dangling pointer scenarios.
pub struct Iter<T> {
    first: NoLifetimeSliceIter<NonNull<T>>,
    second: NoLifetimeSliceIter<NonNull<T>>,
}

impl<T> LazyHeap<T> {
    pub fn new() -> Self {
        Self {
            data: std::collections::VecDeque::new(),
            is_sorted: true,
        }
    }

    #[inline]
    pub fn queue_sort(&mut self) {
        self.is_sorted = false;
    }

    #[inline]
    pub fn front(&self) -> Option<&NonNull<T>> {
        debug_assert!(self.is_sorted, "heap not sorted");
        self.data.front()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> NonNull<T> {
        unsafe {
            let node: NonNull<T> = NonNull::new_unchecked(Box::into_raw(Box::new(value))).cast();

            self.data.push_back(node);
            self.is_sorted = false;

            node
        }
    }

    #[inline]
    pub fn sort_by(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) {
        if self.is_sorted {
            return;
        }

        if self.data.len() > 1 {
            unsafe {
                self.data
                    .make_contiguous()
                    .sort_by(|a, b| compare(a.as_ref(), b.as_ref()));
            }
        }

        self.is_sorted = true;
    }

    #[inline]
    fn unlink_front(&mut self) -> Option<T> {
        let node = self.data.pop_front()?;
        let node = unsafe { Box::from_raw(node.as_ptr()) };
        Some(*node)
    }

    #[inline]
    pub fn pop_front(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Option<T> {
        self.sort_by(compare);
        self.unlink_front()
    }

    #[inline]
    fn unlink_back(&mut self) -> Option<T> {
        let node = self.data.pop_back()?;
        let node = unsafe { Box::from_raw(node.as_ptr()) };
        Some(*node)
    }

    pub fn pop_back(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Option<T> {
        self.sort_by(compare);
        self.unlink_back()
    }

    pub fn get(&self, index: usize) -> Option<&NonNull<T>> {
        self.data.get(index)
    }

    #[inline]
    pub fn remove<F>(&mut self, node: NonNull<T>, compare: F) -> T
    where
        F: Fn(&T, &T) -> std::cmp::Ordering,
    {
        debug_assert!(!self.data.is_empty());

        if self.data.len() == 1 {
            return self.pop_back(compare).unwrap();
        }

        self.sort_by(compare);

        let index = self.data.iter().position(|x| node == *x).unwrap();

        let node = unsafe { self.data.remove(index).unwrap_unchecked() };
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        *boxed_node
    }

    pub fn clear(&mut self) {
        while self.unlink_back().is_some() {}
        self.is_sorted = true;
    }

    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    pub fn iter(&mut self, compare: impl Fn(&T, &T) -> std::cmp::Ordering) -> Iter<T> {
        self.sort_by(compare);

        let (a, b) = self.data.as_slices();

        Iter {
            first: NoLifetimeSliceIter::new(a),
            second: NoLifetimeSliceIter::new(b),
        }
    }
}

impl<T> Drop for LazyHeap<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T>(&'a mut LazyHeap<T>);

        impl<T> Drop for DropGuard<'_, T> {
            fn drop(&mut self) {
                // Continue the same loop we do below. This only runs when a destructor has
                // panicked. If another one panics this will abort.
                while self.0.unlink_back().is_some() {}
            }
        }

        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard(self);
        while guard.0.unlink_back().is_some() {}
        core::mem::forget(guard);
    }
}

impl<T> Iterator for Iter<T> {
    type Item = NonNull<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.first.next() {
            Some(val) => Some(unsafe { *val.as_ptr() }),
            None => {
                core::mem::swap(&mut self.first, &mut self.second);
                self.first.next().map(|x| unsafe { *x.as_ptr() })
            }
        }
    }
}

unsafe impl<T> Send for Iter<T> {}
