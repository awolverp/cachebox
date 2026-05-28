use std::fmt::Write;

use std::sync::atomic;
use std::sync::Arc;

use crate::internal::alias;

/// Tries to hash `arg1`.
///
/// # Safety
/// Pointer must be valid, non-null, live Python objects.
#[inline]
pub unsafe fn pyobject_hash(
    py: pyo3::Python<'_>,
    arg1: *mut pyo3::ffi::PyObject,
) -> pyo3::PyResult<u64> {
    let py_hash = pyo3::ffi::PyObject_Hash(arg1);
    if std::hint::unlikely(py_hash == -1) {
        // SAFETY: PyObject_Hash never returns -1 on success.
        return Err(pyo3::PyErr::take(py).unwrap_unchecked());
    }

    Ok(py_hash as u64)
}

/// Pointer-equality fast path, then Python `==`.
///
/// # Safety
/// Both pointers must be valid, non-null, live Python objects.
#[inline]
pub unsafe fn pyobject_equal(
    py: pyo3::Python<'_>,
    arg1: *mut pyo3::ffi::PyObject,
    arg2: *mut pyo3::ffi::PyObject,
) -> pyo3::PyResult<bool> {
    if std::ptr::eq(arg1, arg2) {
        return Ok(true);
    }

    let boolean = pyo3::ffi::PyObject_RichCompareBool(arg1, arg2, pyo3::ffi::Py_EQ);

    if boolean < 0 {
        Err(pyo3::PyErr::take(py).unwrap_unchecked())
    } else {
        Ok(boolean == 1)
    }
}

/// Calls a Python `getsizeof(key, value) -> int` callable via raw FFI for maximum performance.
///
///
/// # Errors
/// Propagates any Python exception raised by `getsizeof`, and also returns a `PyErr` if:
/// - the return value is not an integer
/// - `PyLong_AsSsize_t` returns `-1` with a live Python exception (overflow / type error)
///
/// # Safety
/// Both pointers must be valid, non-null, live Python objects.
#[inline]
pub unsafe fn call_getsizeof(
    py: pyo3::Python<'_>,
    getsizeof: Option<&alias::PyObject>,
    key: *mut pyo3::ffi::PyObject,
    value: *mut pyo3::ffi::PyObject,
) -> pyo3::PyResult<usize> {
    if getsizeof.is_none() {
        return Ok(1);
    }

    // SAFETY:
    // - All three pointers are valid, live Python objects for the duration of this call.
    // - `PyTuple_New(2)` + `PyTuple_SET_ITEM` is the canonical way to build a
    //   short-lived call tuple without going through Python's allocator twice.
    // - `PyTuple_SET_ITEM` steals a reference, so we `Py_INCREF` key and value first.
    // - We own `args` and decrement it after the call.
    unsafe {
        let getsizeof = getsizeof.unwrap_unchecked();

        let args = pyo3::ffi::PyTuple_New(2);
        if args.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        // PyTuple_SET_ITEM steals the reference, so we need to increment first.
        pyo3::ffi::Py_INCREF(key);
        pyo3::ffi::Py_INCREF(value);
        pyo3::ffi::PyTuple_SET_ITEM(args, 0, key);
        pyo3::ffi::PyTuple_SET_ITEM(args, 1, value);

        let result = pyo3::ffi::PyObject_Call(getsizeof.as_ptr(), args, std::ptr::null_mut());
        pyo3::ffi::Py_DECREF(args);

        if result.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }

        // PyLong_AsSsize_t returns -1 on error.
        // It never allocates and is the fastest int extraction path.
        let size = pyo3::ffi::PyLong_AsSsize_t(result);
        pyo3::ffi::Py_DECREF(result);

        if size == -1 {
            if let Some(err) = pyo3::PyErr::take(py) {
                return Err(err);
            }
        }

        Ok(size as usize)
    }
}

/// Formats an iterator of key-value pairs into a string representation.
///
/// Very useful for implementing `__repr__` methods.
#[inline(never)]
pub fn items_to_str<K, V, I>(items: I, length: usize) -> Result<String, std::fmt::Error>
where
    K: std::fmt::Debug,
    V: std::fmt::Debug,
    I: IntoIterator<Item = (K, V)>,
{
    const EDGE: usize = 50;
    const LIMIT: usize = EDGE * 2;

    let mut out = String::with_capacity(64 + length.min(LIMIT) * 16);
    out.write_char('{')?;

    // Fast path
    if length <= LIMIT {
        for (i, (k, v)) in items.into_iter().enumerate() {
            if i > 0 {
                out.write_str(", ")?;
            }

            write!(out, "{k:?}:{v:?}")?;
        }
        out.write_char('}')?;

        return Ok(out);
    }

    let mut iter = items.into_iter();

    for i in 0..EDGE {
        if let Some((k, v)) = iter.next() {
            if i > 0 {
                out.write_str(", ")?;
            }
            write!(out, "{k:?}:{v:?}")?;
        }
    }

    let mut ring: Vec<(K, V)> = Vec::with_capacity(EDGE);
    let mut head: usize = 0;

    for item in iter {
        if ring.len() < EDGE {
            ring.push(item);
        } else {
            ring[head] = item;
            head = (head + 1) % EDGE;
        }
    }

    let tail_len = ring.len();
    let truncated = length - EDGE - tail_len;
    write!(out, ", ... {truncated} truncated ..., ")?;

    for i in 0..tail_len {
        let (k, v) = &ring[(head + i) % EDGE];
        if i > 0 {
            out.write_str(", ")?;
        }
        write!(out, "{k:?}:{v:?}")?;
    }

    out.write_char('}')?;
    Ok(out)
}

/// Returns the type name of a [`pyo3::ffi::PyObject`].
///
/// Returns `"<unknown>"` on failure.
///
/// # Safety
/// The pointer must be valid, non-null, live Python object.
#[inline(never)]
pub unsafe fn get_type_name<'a>(py: pyo3::Python<'a>, obj: *mut pyo3::ffi::PyObject) -> String {
    use pyo3::types::PyStringMethods;
    use pyo3::types::PyTypeMethods;

    let type_ = pyo3::ffi::Py_TYPE(obj);

    if type_.is_null() {
        String::from("<unknown>")
    } else {
        let obj = pyo3::types::PyType::from_borrowed_type_ptr(py, type_);

        obj.fully_qualified_name()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from("<unknown>"))
    }
}

/// It can use as PyO3 function argument. When an argument is specified, you will get [`OptionalArgument::Defined`],
/// otherwise you will get [`OptionalArgument::Undefined`].
///
/// It can be used instead of [`Option<T>`] to improve performance.
#[derive(Debug)]
pub enum OptionalArgument {
    /// The argument was not provided by the caller.
    Undefined,
    /// The argument was provided and holds the bound Python object.
    Defined(alias::PyObject),
}

impl<'a, 'py> pyo3::FromPyObject<'a, 'py> for OptionalArgument {
    type Error = pyo3::PyErr;

    fn extract(obj: pyo3::Borrowed<'a, 'py, pyo3::PyAny>) -> Result<Self, Self::Error> {
        Ok(Self::Defined(obj.to_owned().unbind()))
    }
}

#[derive(pyo3::FromPyObject, Debug)]
pub enum TimeToLiveArgument {
    Float(f64),
    Timedelta(chrono::TimeDelta),
    DatetimeUtc(chrono::DateTime<chrono::Utc>),
    DatetimeNaive(chrono::NaiveDateTime),
}

impl TimeToLiveArgument {
    #[inline(always)]
    pub fn into_expires_at(self) -> pyo3::PyResult<ExpiresAt> {
        match self {
            Self::Float(secs) => Ok(ExpiresAt::Duration(std::time::Duration::from_secs_f64(
                secs.max(0.0),
            ))),
            Self::Timedelta(delta) => Ok(ExpiresAt::from(delta)),
            Self::DatetimeUtc(until) => Ok(ExpiresAt::from(until)),
            Self::DatetimeNaive(until) => Ok(ExpiresAt::from(until)),
        }
    }

    #[inline(always)]
    pub fn into_duration(self) -> pyo3::PyResult<std::time::Duration> {
        match self {
            Self::Float(secs) => Ok(std::time::Duration::from_secs_f64(secs.max(0.0))),
            Self::Timedelta(delta) => Ok(delta.to_std().unwrap_or(std::time::Duration::ZERO)),
            Self::DatetimeUtc(_) | Self::DatetimeNaive(_) => Err(new_py_error!(
                PyTypeError,
                "expected datetime.timedelta or float, got datetime.datetime"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExpiresAt {
    Duration(std::time::Duration),
    Instant(chrono::DateTime<chrono::Utc>),
}

impl From<std::time::Duration> for ExpiresAt {
    #[inline]
    fn from(value: std::time::Duration) -> Self {
        Self::Duration(value)
    }
}

impl From<chrono::TimeDelta> for ExpiresAt {
    #[inline]
    fn from(value: chrono::TimeDelta) -> Self {
        // Negative or zero timedelta collapses to ZERO duration (expire immediately)
        Self::Duration(value.to_std().unwrap_or(std::time::Duration::ZERO))
    }
}

impl From<chrono::DateTime<chrono::Utc>> for ExpiresAt {
    #[inline]
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Self::Instant(value)
    }
}

impl From<chrono::NaiveDateTime> for ExpiresAt {
    #[inline]
    fn from(value: chrono::NaiveDateTime) -> Self {
        let utc: chrono::DateTime<chrono::Utc> = value
            .and_local_timezone(chrono::Local)
            .single()
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|| value.and_utc());
        Self::Instant(utc)
    }
}

impl From<ExpiresAt> for std::time::SystemTime {
    #[inline]
    fn from(value: ExpiresAt) -> Self {
        match value {
            ExpiresAt::Duration(dur) => std::time::SystemTime::now() + dur,
            ExpiresAt::Instant(until) => until.into(),
        }
    }
}

/// Generation version implementation
///
/// Very useful for checking changes while iteration, like what CPython does;
/// because we can't use lifetimes.
///
/// ```rust
/// let x = GenerationVersion::default();
///
/// x.increment();
/// assert!(x.get() == 1);
/// ```
#[derive(Debug, Clone, Default)]
#[repr(transparent)]
pub struct GenerationVersion(Arc<atomic::AtomicU32>);

impl GenerationVersion {
    #[inline(always)]
    pub fn increment(&self) -> u32 {
        self.0.fetch_add(1, atomic::Ordering::SeqCst)
    }

    #[inline(always)]
    pub fn get(&self) -> u32 {
        self.0.load(atomic::Ordering::Relaxed)
    }
}

/// Precomputed Hash PyObject
///
/// A precomputed hash is a cryptographic hash value that's calculated in advance
/// and stored for later use, rather than being computed on demand when needed.
#[derive(Debug)]
pub struct PrecomputedHashObject {
    object: alias::PyObject,
    hash: u64,
}

impl PrecomputedHashObject {
    /// Creates a new [`PrecomputedHashObject`] with a pre-calculated hash.
    #[inline]
    pub fn with_precomputed_hash(object: alias::PyObject, hash: u64) -> Self {
        Self { object, hash }
    }

    /// Tries to get `object` hash, then creates a new [`PrecomputedHashObject`].
    #[inline]
    pub fn new(py: pyo3::Python<'_>, object: alias::PyObject) -> pyo3::PyResult<Self> {
        let hash = unsafe { pyobject_hash(py, object.as_ptr())? };
        Ok(Self::with_precomputed_hash(object, hash))
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Pointer-equality fast path, then Python `==`.
    #[inline(always)]
    pub fn py_eq(&self, py: pyo3::Python<'_>, other: &Self) -> pyo3::PyResult<bool> {
        unsafe { pyobject_equal(py, self.object.as_ptr(), other.object.as_ptr()) }
    }

    /// Makes a clone of `self`.
    ///
    /// This creates another pointer to the same object, increasing its reference count.
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self {
            object: self.object.clone_ref(py),
            hash: self.hash,
        }
    }
}

impl AsRef<alias::PyObject> for PrecomputedHashObject {
    /// Returns a reference to its pyobject
    #[inline]
    fn as_ref(&self) -> &alias::PyObject {
        &self.object
    }
}

impl From<PrecomputedHashObject> for alias::PyObject {
    /// Consumes `PrecomputedHashObject` and returns its pyobject
    fn from(value: PrecomputedHashObject) -> Self {
        value.object
    }
}
/// Holds and manage `getsizeof` function which is a callable used to measure the
/// size of each key-value pair.
#[derive(pyo3::FromPyObject)]
#[repr(transparent)]
pub struct GetsizeofFunction(Option<alias::PyObject>);

impl GetsizeofFunction {
    /// Creates a new [`GetsizeofFunction`].
    pub fn new(object: Option<alias::PyObject>) -> Self {
        Self(object)
    }

    /// Makes a clone of `self`.
    ///
    /// This creates another pointer to the same object, increasing its reference count.
    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self(self.0.as_ref().map(|x| x.clone_ref(py)))
    }

    /// Calls the wrapped function to get size of the pair key-value.
    #[inline]
    pub fn call(
        &self,
        py: pyo3::Python<'_>,
        key: &alias::PyObject,
        value: &alias::PyObject,
    ) -> pyo3::PyResult<usize> {
        unsafe { call_getsizeof(py, self.0.as_ref(), key.as_ptr(), value.as_ptr()) }
    }
}

impl From<GetsizeofFunction> for Option<alias::PyObject> {
    fn from(value: GetsizeofFunction) -> Self {
        value.0
    }
}

/// Immutable slice iterator without lifetime
///
/// # Safety
/// - You should be sure about lifetimes, and pointers should be alive while this type is alive.
///   Any changes to pointers can cause *Undefined Behaviour*.
/// - It doesn't support `ZST`s.
pub(super) struct RawSliceIter<T> {
    pointer: std::ptr::NonNull<T>,
    index: usize,
    len: usize,
}

impl<T> RawSliceIter<T> {
    /// Creates a new [`RawSliceIter`]
    #[inline]
    pub(super) fn new(slice: &[T]) -> Self {
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
