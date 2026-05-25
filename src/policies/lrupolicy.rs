use crate::hashbrown;
use crate::internal::linked_list;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;

pub use super::common::Handle;
pub use super::common::Shared;

/// A view into an occupied entry in [`LRUPolicy`].
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut LRUPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// Raw bucket pointing to the occupied index.
    bucket: hashbrown::raw::Bucket<linked_list::Cursor<Handle>>,
}

impl traits::OccupiedExt for Occupied<'_> {
    type Handle = Handle;
    type Shared = Shared;

    fn replace(self, new: Self::Handle) -> Self::Handle {
        self.shared.generation_version().increment();

        unsafe {
            let mut cursor = *self.bucket.as_ref();

            self.policy.currsize = self
                .policy
                .currsize
                .saturating_sub(cursor.element().size())
                .saturating_add(new.size());

            let old = std::mem::replace(cursor.element_mut(), new);
            cursor.move_to_back(&mut self.policy.list);

            old
        }
    }

    #[inline]
    fn remove(self) -> Self::Handle {
        self.shared.generation_version().increment();

        let (cursor, _) = unsafe { self.policy.table.remove(self.bucket) };
        let item = unsafe { cursor.unlink(&mut self.policy.list) };

        self.policy.currsize = self.policy.currsize.saturating_sub(item.size());
        item
    }
}

/// A view into a vacant slot in [`LRUPolicy`].
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut LRUPolicy,
    /// The shared configuration
    shared: &'a Shared,
}

impl traits::VacantExt for Vacant<'_> {
    type Handle = Handle;
    type Shared = Shared;

    #[inline]
    fn would_exceed(&self, extra_size: usize) -> bool {
        self.policy.currsize.saturating_add(extra_size) > self.shared.maxsize()
    }

    #[inline]
    fn evict(&mut self, py: pyo3::Python) -> pyo3::PyResult<()> {
        self.policy.evict(py, self.shared)?;
        Ok(())
    }

    fn insert(self, handle: Self::Handle) {
        self.shared.generation_version().increment();

        self.policy.currsize = self.policy.currsize.saturating_add(handle.size());

        let hash = handle.key().hash();
        let cursor = self.policy.list.push_back(handle);

        self.policy
            .table
            .insert(hash, cursor, |x| unsafe { x.element().key().hash() });
    }
}

pub struct LRUPolicy {
    /// Maps each key to its node pointer into [`LRUPolicy::list`], enabling O(1) lookups.
    table: hashbrown::raw::RawTable<linked_list::Cursor<Handle>>,

    /// A doubly-linked list, which holds cached handles, providing O(1) pops (front/back) and pushes (front/back).
    list: linked_list::LinkedList<Handle>,

    /// Running total of all stored handles' sizes, maintained incrementally.
    currsize: usize,
}

impl LRUPolicy {
    /// Creates a new [`LRUPolicy`].
    ///
    /// The underlying hash map is pre-allocated to hold at least `capacity` entries
    /// without reallocation.
    pub fn new(capacity: usize) -> Self {
        Self {
            table: hashbrown::raw::RawTable::with_capacity(capacity),
            list: linked_list::LinkedList::new(),
            currsize: 0,
        }
    }

    #[inline]
    pub fn table(&self) -> &hashbrown::raw::RawTable<linked_list::Cursor<Handle>> {
        &self.table
    }

    #[inline]
    pub fn list(&self) -> &linked_list::LinkedList<Handle> {
        &self.list
    }

    #[inline]
    pub fn peek(
        &self,
        py: pyo3::Python,
        key: &utils::PrecomputedHashObject,
    ) -> pyo3::PyResult<Option<&Handle>> {
        unsafe {
            let bucket = self
                .table
                .find(key.hash(), |cursor| key.py_eq(py, cursor.element().key()))?;

            Ok(bucket.map(|x| x.as_ref().element()))
        }
    }
}

impl PolicyExt for LRUPolicy {
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
        key: &<Self::Handle as super::traits::HandleExt>::Key,
    ) -> pyo3::PyResult<Option<&Self::Handle>> {
        unsafe {
            let bucket = self
                .table
                .get(key.hash(), |cursor| key.py_eq(py, cursor.element().key()))?;

            match bucket {
                Some(cursor) => {
                    cursor.move_to_back(&mut self.list);
                    Ok(Some(cursor.element()))
                }
                None => Ok(None),
            }
        }
    }

    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>> {
        let eq =
            |cursor: &linked_list::Cursor<Handle>| unsafe { key.py_eq(py, cursor.element().key()) };

        match self.table.find(key.hash(), eq)? {
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
                };
                Ok(traits::PolicyEntry::Vacant(result))
            }
        }
    }

    fn evict(&mut self, _py: pyo3::Python, shared: &Self::Shared) -> pyo3::PyResult<Self::Handle> {
        {
            let front_cursor = match self.list.cursor_front() {
                Some(x) => x,
                None => return Err(new_py_error!(PyKeyError, "cache is empty")),
            };

            let hash = unsafe { front_cursor.element().key().hash() };

            shared.generation_version().increment();
            self.table
                .remove_entry(hash, |cursor| Ok::<_, pyo3::PyErr>(*cursor == front_cursor))
                .expect("evict: key not found in table.");
        }

        let handle = unsafe { self.list.pop_front().unwrap_unchecked() };
        self.currsize = self.currsize.saturating_sub(handle.size());
        Ok(handle)
    }

    #[inline]
    fn shrink_to_fit(&mut self, _shared: &Self::Shared) {
        self.table
            .shrink_to(0, |cursor| unsafe { cursor.element().key().hash() });
    }

    #[inline]
    fn clear(&mut self, shared: &Self::Shared) {
        if self.list.is_empty() {
            return;
        }

        shared.generation_version().increment();
        self.table.clear_no_drop();
        self.list.clear();
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

            iterator.all(|cursor_1| {
                let handle_1 = cursor_1.element();

                let result = other.table.get(handle_1.key().hash(), |cursor| {
                    handle_1.key().py_eq(py, cursor.element().key())
                });

                match result {
                    Err(e) => {
                        error = Some(e);
                        // Return false to break the `.all` loop
                        false
                    }
                    Ok(None) => false,
                    Ok(Some(cursor_2)) => {
                        let handle_2 = cursor_2.element();

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
        let mut table = hashbrown::raw::RawTable::with_capacity(self.list.len());
        let mut entries = linked_list::LinkedList::new();

        unsafe {
            for cursor in self.list.iter() {
                let cloned_handle = cursor.element().clone_ref(py);
                let new_cursor = entries.push_back(cloned_handle);
                table.insert_no_grow(new_cursor.element().key().hash(), new_cursor);
            }
        }

        Self {
            table,
            list: entries,
            currsize: self.currsize,
        }
    }
}
