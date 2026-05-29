use crate::hashbrown;
use crate::internal::alias;
use crate::internal::lazyheap;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;

pub use crate::policies::common::Shared;
use crate::policies::traits::SharedExt;

/// Compares two items by `expires_at`, placing `None` values last.
macro_rules! compare_fn {
    () => {
        |a, b| {
            a.expires_at
                .is_none()
                .cmp(&b.expires_at.is_none())
                .then_with(|| a.expires_at.cmp(&b.expires_at))
        }
    };
}

/// A key-value pair with a precomputed hash and combined size.
pub struct ExpiringHandle {
    /// The cache key together with its precomputed hash, avoiding repeated
    /// Python hash calls during table lookups.
    key: utils::PrecomputedHashObject,
    /// The cached value associated with this key.
    value: alias::PyObject,
    /// Size of the key and value as reported by `getsizeof`.
    size: usize,
    /// Configured ttl for handle. `None` means has no ttl.
    expires_at: Option<std::time::SystemTime>,
}

impl ExpiringHandle {
    /// Creates a new [`Handle`], which calculates the precomputed hash itself.
    #[inline]
    pub fn new(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        expires_at: Option<utils::ExpiresAt>,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        Self::with_precomputed_hash_key(
            py,
            getsizeof,
            expires_at,
            utils::PrecomputedHashObject::new(py, key)?,
            value,
        )
    }

    /// Creates a new [`Handle`] from an already-hashed key.
    ///
    /// Prefer this over [`Handle::new`] when the caller has already paid the cost
    /// of computing the hash (e.g. during a table lookup that preceded insertion).
    #[inline]
    pub fn with_precomputed_hash_key(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        expires_at: Option<utils::ExpiresAt>,
        key: utils::PrecomputedHashObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        let size = getsizeof.call(py, key.as_ref(), &value)?;
        Ok(Self {
            key,
            value,
            size,
            expires_at: expires_at.map(Into::into),
        })
    }

    /// Consumes `self` and returns the [`utils::PrecomputedHashObject`].
    #[inline]
    pub fn into_key(self) -> utils::PrecomputedHashObject {
        self.key
    }

    /// Returns a reference to the value.
    #[inline]
    pub fn value(&self) -> &alias::PyObject {
        &self.value
    }

    /// Consumes `self` and returns the value of the pair.
    #[inline]
    pub fn into_value(self) -> alias::PyObject {
        self.value
    }

    /// Consumes `self` and returns the pair.
    #[inline]
    pub fn into_pair(self) -> (utils::PrecomputedHashObject, alias::PyObject) {
        (self.key, self.value)
    }

    #[inline]
    pub fn expires_at(&self) -> Option<std::time::SystemTime> {
        self.expires_at
    }

    #[inline]
    pub fn is_expired(&self, now: std::time::SystemTime) -> bool {
        self.expires_at.map(|x| x <= now).unwrap_or_default()
    }

    /// Makes a clone of self.
    ///
    /// This creates another pointer to the same object, increasing its reference count.
    #[inline]
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self {
            key: self.key.clone_ref(py),
            value: self.value.clone_ref(py),
            size: self.size,
            expires_at: self.expires_at,
        }
    }
}

impl HandleExt for ExpiringHandle {
    type Key = utils::PrecomputedHashObject;

    #[inline(always)]
    fn key(&self) -> &utils::PrecomputedHashObject {
        &self.key
    }

    #[inline(always)]
    fn size(&self) -> usize {
        self.size
    }
}

/// A view into an occupied entry in [`VTTLPolicy`].
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut VTTLPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// Raw bucket pointing to the occupied index.
    bucket: hashbrown::raw::Bucket<lazyheap::Cursor<ExpiringHandle>>,
}

impl traits::OccupiedExt for Occupied<'_> {
    type Handle = ExpiringHandle;
    type Shared = Shared;

    fn replace(self, new: Self::Handle) -> Self::Handle {
        // Here we don't need to increment generation version
        // self.shared.generation_version().increment();

        unsafe {
            let cursor = self.bucket.as_mut();

            self.policy.currsize = self
                .policy
                .currsize
                .saturating_sub(cursor.element().size())
                .saturating_add(new.size());

            let old = std::mem::replace(cursor.element_mut(), new);

            self.policy.heap.mark_unsorted();
            old
        }
    }

    #[inline]
    fn remove(self) -> Self::Handle {
        self.shared.generation_version().increment();

        let (cursor, _) = unsafe { self.policy.table.remove(self.bucket) };
        let item = self.policy.heap.remove(cursor, compare_fn!());

        self.policy.currsize = self.policy.currsize.saturating_sub(item.size());
        item
    }
}
/// A view into a vacant slot in [`VTTLPolicy`].
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut VTTLPolicy,
    /// The shared configuration
    shared: &'a Shared,
}

impl traits::VacantExt for Vacant<'_> {
    type Handle = ExpiringHandle;
    type Shared = Shared;

    #[inline]
    fn would_exceed(&self, extra_size: usize) -> bool {
        self.policy.currsize.saturating_add(extra_size) > self.shared.maxsize()
    }

    #[inline]
    fn evict(&mut self) -> pyo3::PyResult<()> {
        self.policy.evict(self.shared)?;
        Ok(())
    }

    fn insert(self, handle: Self::Handle) {
        self.shared.generation_version().increment();

        self.policy.currsize = self.policy.currsize.saturating_add(handle.size());

        let hash = handle.key().hash();
        let cursor = self.policy.heap.push(handle);

        self.policy
            .table
            .insert(hash, cursor, |x| unsafe { x.element().key().hash() });
    }
}

pub struct VTTLPolicy {
    // Fields are same as `LFUPolicy`
    table: hashbrown::raw::RawTable<lazyheap::Cursor<ExpiringHandle>>,
    heap: lazyheap::LazyHeap<ExpiringHandle>,
    currsize: usize,
}

impl VTTLPolicy {
    /// Creates a new [`VTTLPolicy`].
    ///
    /// The underlying hash map is pre-allocated to hold at least `capacity` entries
    /// without reallocation.
    pub fn new(capacity: usize) -> Self {
        Self {
            table: hashbrown::raw::RawTable::with_capacity(capacity),
            heap: lazyheap::LazyHeap::new(),
            currsize: 0,
        }
    }

    #[inline]
    pub fn table(&self) -> &hashbrown::raw::RawTable<lazyheap::Cursor<ExpiringHandle>> {
        &self.table
    }

    #[inline]
    pub fn heap(&self) -> &lazyheap::LazyHeap<ExpiringHandle> {
        &self.heap
    }

    #[inline]
    pub fn iter(&mut self, gv: &utils::GenerationVersion) -> lazyheap::RawIter<ExpiringHandle> {
        self.expire(gv);

        // We don't want to intrupt other iterators with no reason
        // so need to manually call sort_by to only intrupt them on changes.
        if self.heap.sort_by(compare_fn!()) {
            gv.increment();
        }

        self.heap.iter(compare_fn!())
    }

    pub fn expire(&mut self, gv: &utils::GenerationVersion) {
        let now = std::time::SystemTime::now();

        while let Some(cursor) = self.heap.front(compare_fn!()) {
            let handle = unsafe { cursor.element() };

            if !handle.is_expired(now) {
                break;
            }

            self.table
                .remove_entry(handle.key.hash(), |x| {
                    Ok::<_, pyo3::PyErr>(x.as_ptr() == cursor.as_ptr())
                })
                .unwrap();

            drop(cursor);

            gv.increment();

            let handle = self.heap.pop_front(compare_fn!()).unwrap();
            self.currsize = self.currsize.saturating_sub(handle.size);
        }
    }
}

impl PolicyExt for VTTLPolicy {
    type Shared = Shared;
    type Handle = ExpiringHandle;

    type Occupied<'a>
        = Occupied<'a>
    where
        Self: 'a;

    type Vacant<'a>
        = Vacant<'a>
    where
        Self: 'a;
    const PICKLE_SIZE: usize = 1;

    #[inline]
    fn current_size(&self) -> usize {
        self.currsize
    }

    #[inline]
    fn get(
        &mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
    ) -> pyo3::PyResult<Option<&Self::Handle>> {
        let cursor = self
            .table
            .get_mut(key.hash(), |x| unsafe { key.py_eq(py, &x.element().key) })?;

        match cursor {
            Some(cursor) => {
                let handle = unsafe { cursor.element() };

                if handle.is_expired(std::time::SystemTime::now()) {
                    Ok(None)
                } else {
                    Ok(Some(handle))
                }
            }
            None => Ok(None),
        }
    }

    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>> {
        self.expire(shared.generation_version());

        let eq = |cursor: &lazyheap::Cursor<Self::Handle>| unsafe {
            key.py_eq(py, cursor.element().key())
        };

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

    fn evict(&mut self, shared: &Self::Shared) -> pyo3::PyResult<Self::Handle> {
        {
            let front_cursor = self
                .heap
                .front(compare_fn!())
                .ok_or_else(|| new_py_error!(PyKeyError, "cache is empty"))?;

            self.table
                .remove_entry(unsafe { front_cursor.element().key.hash() }, |x| {
                    Ok::<_, pyo3::PyErr>(std::ptr::eq(front_cursor.as_ptr(), x.as_ptr()))
                })?
                .expect("evict: item not found in table");
        }

        shared.generation_version().increment();

        let handle = self.heap.pop_front(compare_fn!()).unwrap();

        self.currsize = self.currsize.saturating_sub(handle.size);
        Ok(handle)
    }

    fn clear(&mut self, shared: &Self::Shared) {
        if self.heap.is_empty() {
            return;
        }

        shared.generation_version().increment();
        self.table.clear_no_drop();
        self.heap.clear();
        self.currsize = 0;
    }

    fn shrink_to_fit(&mut self, shared: &Self::Shared) {
        shared.generation_version().increment();

        self.table
            .shrink_to(0, |x| unsafe { x.element().key.hash() });

        self.heap.shrink_to_fit();
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
        let now = std::time::SystemTime::now();

        let result = unsafe {
            self.table.iter().all(|x| {
                let handle = x.as_ref().element();

                if handle.is_expired(now) {
                    return true;
                }

                let key = handle.key();

                match other
                    .table
                    .get(key.hash(), |c| key.py_eq(py, c.element().key()))
                {
                    Err(e) => {
                        error = Some(e);
                        false
                    }
                    Ok(None) => false,
                    Ok(Some(cursor)) => {
                        match utils::pyobject_equal(
                            py,
                            handle.value.as_ptr(),
                            cursor.element().value.as_ptr(),
                        ) {
                            Ok(eq) => eq,
                            Err(e) => {
                                error = Some(e);
                                false
                            }
                        }
                    }
                }
            })
        };

        error.map_or(Ok(result), Err)
    }

    fn clone_ref(&mut self, py: pyo3::Python) -> Self {
        let mut table = hashbrown::raw::RawTable::with_capacity(self.table.len());
        let mut heap = lazyheap::LazyHeap::new();

        unsafe {
            for cursor in self.heap.iter(compare_fn!()) {
                let cloned_handle = cursor.element().clone_ref(py);
                let new_cursor = heap.push(cloned_handle);
                table.insert_no_grow(new_cursor.element().key().hash(), new_cursor);
            }
        }

        Self {
            table,
            heap,
            currsize: self.currsize,
        }
    }
}
