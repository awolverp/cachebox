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
    /// Global time-to-live for cache entries. This is for *TTL* implementations.
    global_ttl: Option<std::time::Duration>,
}

impl Shared {
    /// Creates a new [`Shared`].
    #[inline]
    pub fn new(maxsize: usize, getsizeof: Option<alias::PyObject>) -> Self {
        Self::with_ttl(maxsize, getsizeof, None)
    }

    /// Creates a new [`Shared`] with configured TTL.
    #[inline]
    pub fn with_ttl(
        maxsize: usize,
        getsizeof: Option<alias::PyObject>,
        ttl: Option<std::time::Duration>,
    ) -> Self {
        Self {
            maxsize: safe_non_zero!(maxsize),
            gv: utils::GenerationVersion::default(),
            getsizeof: utils::GetsizeofFunction::new(getsizeof),
            global_ttl: ttl,
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

    #[inline]
    fn global_ttl(&self) -> Option<std::time::Duration> {
        self.global_ttl
    }

    fn clone_ref(&self, py: pyo3::Python) -> Self {
        Self {
            maxsize: self.maxsize,
            gv: Default::default(),
            getsizeof: self.getsizeof.clone_ref(py),
            global_ttl: self.global_ttl,
        }
    }
}
