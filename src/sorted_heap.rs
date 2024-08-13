use std::ptr::NonNull;

pub struct SortedHeap<T>(pub Vec<NonNull<Entry<T>>>, pub bool);

pub struct Entry<T>(T);

pub struct Iter<T> {
    slice: *const NonNull<Entry<T>>,
    index: usize,
    len: usize,
}

impl<T> SortedHeap<T> {
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new(), true)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn push(&mut self, value: T) -> NonNull<Entry<T>> {
        unsafe {
            let node = NonNull::new_unchecked(Box::into_raw(Box::new(Entry(value))));

            self.0.push(node);
            self.1 = false;

            node
        }
    }

    #[inline]
    pub fn sort<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        if !self.1 {
            if self.0.len() > 1 {
                unsafe {
                    self.0
                        .sort_by(|a, b| compare(&(*a.as_ptr()).0, &(*b.as_ptr()).0));
                }
            }

            self.1 = true;
        }
    }

    #[inline]
    fn unlink_first(&mut self) -> Option<T> {
        if self.0.is_empty() {
            return None;
        }

        let node = self.0.remove(0);
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        Some(boxed_node.0)
    }

    pub fn pop_front<F>(&mut self, compare: F) -> Option<T>
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.sort(compare);
        self.unlink_first()
    }

    #[inline]
    fn unlink_last(&mut self) -> Option<T> {
        let node = self.0.pop()?;
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        Some(boxed_node.0)
    }

    pub fn pop_back<F>(&mut self, compare: F) -> Option<T>
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.sort(compare);
        self.unlink_last()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&NonNull<Entry<T>>> {
        self.0.get(index)
    }

    pub fn remove<F>(&mut self, node: NonNull<Entry<T>>, compare: F) -> T
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        debug_assert!(!self.0.is_empty());

        if self.0.len() == 1 {
            return self.pop_back(compare).unwrap();
        }

        self.sort(compare);

        let index = self.0.iter().position(|x| node == *x).unwrap();

        let node = self.0.remove(index);
        let boxed_node = unsafe { Box::from_raw(node.as_ptr()) };
        boxed_node.0
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            slice: self.0.as_ptr(),
            index: 0,
            len: self.0.len(),
        }
    }

    pub fn clear(&mut self) {
        while self.unlink_last().is_some() {}
    }
}

impl<T> Drop for SortedHeap<T> {
    fn drop(&mut self) {
        struct DropGuard<'a, T>(&'a mut SortedHeap<T>);

        impl<'a, T> Drop for DropGuard<'a, T> {
            fn drop(&mut self) {
                // Continue the same loop we do below. This only runs when a destructor has
                // panicked. If another one panics this will abort.
                while self.0.unlink_last().is_some() {}
            }
        }

        // Wrap self so that if a destructor panics, we can try to keep looping
        let guard = DropGuard(self);
        while guard.0.unlink_last().is_some() {}
        core::mem::forget(guard);
    }
}

impl<T> AsRef<T> for Entry<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Entry<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Iterator for Iter<T> {
    type Item = NonNull<Entry<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            None
        } else {
            let value = unsafe { self.slice.add(self.index) };
            self.index += 1;
            Some(unsafe { *value })
        }
    }
}

// because we use it in Mutex
unsafe impl<T> Sync for Iter<T> {}

// because we use it in Mutex
unsafe impl<T> Send for Iter<T> {}
