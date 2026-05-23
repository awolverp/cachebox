//! Common implementations accross multiple policies

use crate::internal::alias;
use crate::internal::utils;
use crate::policies::traits;

/// A key-value pair with a precomputed hash and combined size.
pub struct Handle {
    /// The cache key together with its precomputed hash, avoiding repeated
    /// Python hash calls during table lookups.
    key: utils::PrecomputedHashObject,
    /// The cached value associated with this key.
    value: alias::PyObject,
    /// Size of the key and value as reported by `getsizeof`.
    size: usize,
}

impl Handle {
    /// Creates a new [`Handle`], which calculates the precomputed hash itself.
    #[inline]
    pub fn new(
        py: pyo3::Python<'_>,
        getsizeof: &utils::GetsizeofFunction,
        key: alias::PyObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        Self::with_precomputed_hash_key(
            py,
            getsizeof,
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
        key: utils::PrecomputedHashObject,
        value: alias::PyObject,
    ) -> pyo3::PyResult<Self> {
        let size = getsizeof.call(py, key.as_ref(), &value)?;
        Ok(Self { key, value, size })
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
        }
    }
}

impl traits::HandleExt for Handle {
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

/// Shared variables which should separated from Mutex
pub struct Shared {
    // Hard upper bound on `currsize`.
    maxsize: std::num::NonZeroUsize,
    /// Monotonically incrementing counter bumped on every structural mutation
    gv: utils::GenerationVersion,
    /// Callable used to measure size of each key-value pair.
    getsizeof: utils::GetsizeofFunction,
}

impl Shared {
    /// Creates a new [`Shared`].
    #[inline]
    pub fn new(maxsize: usize, getsizeof: Option<alias::PyObject>) -> Self {
        Self {
            maxsize: safe_non_zero!(maxsize),
            gv: utils::GenerationVersion::default(),
            getsizeof: utils::GetsizeofFunction::new(getsizeof),
        }
    }
}

impl traits::SharedExt for Shared {
    #[inline]
    fn maxsize(&self) -> usize {
        self.maxsize.get()
    }

    #[inline]
    fn generation_version(&self) -> &utils::GenerationVersion {
        &self.gv
    }

    #[inline]
    fn getsizeof(&self) -> &utils::GetsizeofFunction {
        &self.getsizeof
    }

    fn clone_ref(&self, py: pyo3::Python) -> Self {
        Self {
            maxsize: self.maxsize,
            gv: Default::default(),
            getsizeof: self.getsizeof.clone_ref(py),
        }
    }
}

/// Immutable slice iterator without lifetime
///
/// # Safety
/// - You should be sure about lifetimes, and pointers should be alive while this type is alive.
///   Any changes to pointers can cause *Undefined Behaviour*.
/// - It doesn't support `ZST`s.
struct RawSliceIter<T> {
    pointer: std::ptr::NonNull<T>,
    index: usize,
    len: usize,
}

impl<T> RawSliceIter<T> {
    /// Creates a new [`RawSliceIter`]
    #[inline]
    fn new(slice: &[T]) -> Self {
        let pointer: std::ptr::NonNull<T> = std::ptr::NonNull::from(slice).cast();

        Self {
            pointer,
            index: 0,
            len: slice.len(),
        }
    }
}

impl<T> Iterator for RawSliceIter<T> {
    type Item = std::ptr::NonNull<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            None
        } else {
            let value = unsafe { self.pointer.add(self.index) };
            self.index += 1;
            Some(value)
        }
    }
}

unsafe impl<T: Sync> Send for RawSliceIter<T> {}
unsafe impl<T: Sync> Sync for RawSliceIter<T> {}

/// Raw iterator for [`VecDeque`] which doesn't have lifetime.
///
/// # Safety
/// You should track changes of [`VecDeque`] yourself.
pub struct RawVecDequeIter<T> {
    first: RawSliceIter<T>,
    second: RawSliceIter<T>,
}

impl<T> RawVecDequeIter<T> {
    /// Creates a new [`RawVecDequeIter`]
    #[inline]
    pub fn new(first: &[T], second: &[T]) -> Self {
        Self {
            first: RawSliceIter::new(first),
            second: RawSliceIter::new(second),
        }
    }
}

impl<T> Iterator for RawVecDequeIter<T> {
    type Item = std::ptr::NonNull<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.first.next() {
            Some(val) => Some(val),
            None => {
                std::mem::swap(&mut self.first, &mut self.second);
                self.first.next()
            }
        }
    }
}
