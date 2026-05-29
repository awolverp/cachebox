use crate::hashbrown;
use crate::internal::alias;
use crate::internal::lazyheap;
use crate::internal::utils;
use crate::policies::traits;
use crate::policies::traits::HandleExt;
use crate::policies::traits::PolicyExt;
use crate::policies::traits::SharedExt;

pub use crate::policies::common::Shared;

macro_rules! compare_fn {
    () => {
        |x, y| x.frequency.cmp(&y.frequency)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Frequency(u128);

impl Frequency {
    #[inline(always)]
    fn increment(&mut self) {
        self.0 = self.0.saturating_add(1)
    }
}

/// Same as [`Handle`](struct@super::common::Handle), but with a frequency counter.
pub struct FrequencyHandle {
    key: utils::PrecomputedHashObject,
    value: alias::PyObject,
    size: usize,
    frequency: Frequency,
}

impl FrequencyHandle {
    /// Creates a new [`FrequencyHandle`] with an initial frequency (always is zero, except
    /// in loading pickle states).
    #[inline]
    pub fn new(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        key: alias::PyObject,
        value: alias::PyObject,
        frequency: u128,
    ) -> pyo3::PyResult<Self> {
        Self::with_precomputed_hash_key(
            py,
            getsizeof,
            utils::PrecomputedHashObject::new(py, key)?,
            value,
            frequency,
        )
    }

    /// Creates a new [`FrequencyHandle`] from an already-hashed key,
    /// with an initial frequency (always is zero, except in loading pickle states).
    #[inline]
    pub fn with_precomputed_hash_key(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        key: utils::PrecomputedHashObject,
        value: alias::PyObject,
        frequency: u128,
    ) -> pyo3::PyResult<Self> {
        let size = getsizeof.call(py, key.as_ref(), &value)?;
        Ok(Self {
            key,
            value,
            size,
            frequency: Frequency(frequency),
        })
    }

    /// Returns the frequency.
    #[inline]
    pub fn frequency(&self) -> u128 {
        self.frequency.0
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

    /// Makes a clone of self.
    ///
    /// This creates another pointer to the same object, increasing its reference count.
    #[inline]
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self {
            key: self.key.clone_ref(py),
            value: self.value.clone_ref(py),
            size: self.size,
            frequency: self.frequency,
        }
    }
}

impl HandleExt for FrequencyHandle {
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

/// A view into an occupied entry in [`LFUPolicy`].
pub struct Occupied<'a> {
    /// The parent storage that owns the hash table.
    policy: &'a mut LFUPolicy,
    /// The shared configuration
    shared: &'a Shared,
    /// Raw bucket pointing to the occupied index.
    bucket: hashbrown::raw::Bucket<lazyheap::Cursor<FrequencyHandle>>,
}

impl traits::OccupiedExt for Occupied<'_> {
    type Handle = FrequencyHandle;
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

            cursor.element_mut().frequency.increment();
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

/// A view into a vacant slot in [`LFUPolicy`].
pub struct Vacant<'a> {
    /// The parent policy that owns the hash table.
    policy: &'a mut LFUPolicy,
    /// The shared configuration
    shared: &'a Shared,
}

impl traits::VacantExt for Vacant<'_> {
    type Handle = FrequencyHandle;
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

pub struct LFUPolicy {
    /// Maps each key to its node pointer into [`LFUPolicy::entries`], enabling O(1) lookups.
    table: hashbrown::raw::RawTable<lazyheap::Cursor<FrequencyHandle>>,

    /// A lazy binary heap.
    heap: lazyheap::LazyHeap<FrequencyHandle>,

    /// Running total of all stored handles' sizes, maintained incrementally.
    currsize: usize,
}

impl LFUPolicy {
    /// Creates a new [`LFUPolicy`].
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
    pub fn table(&self) -> &hashbrown::raw::RawTable<lazyheap::Cursor<FrequencyHandle>> {
        &self.table
    }

    #[inline]
    pub fn heap(&self) -> &lazyheap::LazyHeap<FrequencyHandle> {
        &self.heap
    }

    #[inline]
    pub fn iter(&mut self, gv: &utils::GenerationVersion) -> lazyheap::RawIter<FrequencyHandle> {
        // We don't want to intrupt other iterators with no reason
        // so need to manually call sort_by to only intrupt them on changes.
        if self.heap.sort_by(compare_fn!()) {
            gv.increment();
        }

        self.heap.iter(compare_fn!())
    }

    #[inline]
    pub fn least_frequently_used(
        &mut self,
        py: pyo3::Python,
        n: usize,
        gv: &utils::GenerationVersion,
    ) -> Option<utils::PrecomputedHashObject> {
        if self.heap.sort_by(compare_fn!()) {
            gv.increment();
        }

        self.heap
            .get(n)
            .map(|cursor| unsafe { cursor.element().key().clone_ref(py) })
    }

    #[inline]
    pub fn peek(
        &self,
        py: pyo3::Python,
        key: &utils::PrecomputedHashObject,
    ) -> pyo3::PyResult<Option<&FrequencyHandle>> {
        unsafe {
            let bucket = self
                .table
                .find(key.hash(), |cursor| key.py_eq(py, &cursor.element().key))?;

            Ok(bucket.map(|x| x.as_ref().element()))
        }
    }
}

impl PolicyExt for LFUPolicy {
    type Shared = Shared;
    type Handle = FrequencyHandle;

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
        key: &<Self::Handle as traits::HandleExt>::Key,
    ) -> pyo3::PyResult<Option<&Self::Handle>> {
        let cursor = self
            .table
            .get_mut(key.hash(), |x| unsafe { key.py_eq(py, &x.element().key) })?;

        match cursor {
            Some(cursor) => unsafe {
                // increment frequency
                cursor.element_mut().frequency.increment();

                Ok(Some(cursor.element()))
            },
            None => Ok(None),
        }
    }

    fn entry<'a>(
        &'a mut self,
        py: pyo3::Python,
        key: &<Self::Handle as HandleExt>::Key,
        shared: &'a Self::Shared,
    ) -> pyo3::PyResult<traits::PolicyEntry<Self::Occupied<'a>, Self::Vacant<'a>>> {
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

        let result = unsafe {
            self.table.iter().all(|x| {
                let handle = x.as_ref().element();

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
