use std::ptr;

use crate::internal::alias;

pub enum PyPickleVal<'a> {
    Owned(alias::PyObject),
    Borrowed(&'a alias::PyObject),
    Str(&'a str),
    Unsigned(usize),
    Signed(isize),
    Float(f64),
    Bool(bool),
    None,
}

impl From<usize> for PyPickleVal<'static> {
    #[inline]
    fn from(v: usize) -> Self {
        PyPickleVal::Unsigned(v)
    }
}
impl From<isize> for PyPickleVal<'static> {
    #[inline]
    fn from(v: isize) -> Self {
        PyPickleVal::Signed(v)
    }
}
impl From<f64> for PyPickleVal<'static> {
    fn from(v: f64) -> Self {
        PyPickleVal::Float(v)
    }
}
impl From<std::time::Duration> for PyPickleVal<'static> {
    #[inline]
    fn from(v: std::time::Duration) -> Self {
        v.as_secs_f64().into()
    }
}
impl From<bool> for PyPickleVal<'static> {
    #[inline]
    fn from(v: bool) -> Self {
        PyPickleVal::Bool(v)
    }
}
impl<'a> From<&'a str> for PyPickleVal<'a> {
    #[inline]
    fn from(v: &'a str) -> Self {
        PyPickleVal::Str(v)
    }
}
impl<'a> From<&'a alias::PyObject> for PyPickleVal<'a> {
    #[inline]
    fn from(v: &'a alias::PyObject) -> Self {
        PyPickleVal::Borrowed(v)
    }
}
impl From<alias::PyObject> for PyPickleVal<'static> {
    #[inline]
    fn from(v: alias::PyObject) -> Self {
        PyPickleVal::Owned(v)
    }
}
impl<'a, I> From<Option<I>> for PyPickleVal<'a>
where
    I: Into<PyPickleVal<'a>>,
{
    #[inline]
    fn from(value: Option<I>) -> Self {
        match value {
            Some(x) => x.into(),
            None => Self::None,
        }
    }
}

// private methods
impl<'a> PyPickleVal<'a> {
    /// Allocate a fresh owned Python object.
    ///
    /// # Safety
    /// The caller is responsible for exactly one `Py_DECREF` (or transferring ownership to a container).
    unsafe fn into_py_raw(self, py: pyo3::Python<'_>) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let ptr = match self {
            Self::Owned(v) => v.into_ptr(),
            Self::Borrowed(v) => {
                let ptr = v.as_ptr();
                pyo3::ffi::Py_INCREF(ptr);
                ptr
            }
            Self::Unsigned(v) => pyo3::ffi::PyLong_FromSize_t(v),
            Self::Signed(v) => pyo3::ffi::PyLong_FromSsize_t(v),
            Self::Float(v) => pyo3::ffi::PyFloat_FromDouble(v),
            Self::Bool(v) => {
                // Py_True / Py_False are singletons; INCREF to hand out our own ref.
                let raw = if v {
                    pyo3::ffi::Py_True()
                } else {
                    pyo3::ffi::Py_False()
                };
                pyo3::ffi::Py_INCREF(raw);
                raw
            }
            Self::Str(v) => pyo3::ffi::PyUnicode_FromStringAndSize(
                v.as_ptr() as *const std::os::raw::c_char,
                v.len() as isize,
            ),
            Self::None => {
                let none = pyo3::ffi::Py_None();
                pyo3::ffi::Py_INCREF(none);
                none
            }
        };

        if ptr.is_null() {
            Err(pyo3::PyErr::fetch(py))
        } else {
            Ok(ptr)
        }
    }
}

/// A finalised pickle state - an immutable wrapper around a Python tuple.
///
/// Construct with [`Pickle::builder`].
#[repr(transparent)]
pub struct Pickle(alias::PyObject);

impl Pickle {
    /// Begin building a top-level pickle tuple with exactly `size` slots.
    #[inline]
    pub fn builder(py: pyo3::Python<'_>, size: usize) -> pyo3::PyResult<PickleBuilder> {
        PickleBuilder::new(py, size as isize)
    }

    /// Borrow the inner [`alias::PyObject`] without consuming `self`.
    #[inline]
    pub fn as_object(&self) -> &alias::PyObject {
        &self.0
    }
}

impl std::ops::Deref for Pickle {
    type Target = alias::PyObject;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<alias::PyObject> for Pickle {
    #[inline]
    fn as_ref(&self) -> &alias::PyObject {
        &self.0
    }
}

impl From<Pickle> for alias::PyObject {
    #[inline]
    fn from(v: Pickle) -> Self {
        v.0
    }
}

/// Most of builders expose the same `push` / `push_tuple` / `push_list` / `push_dict` surface.
/// Rather than repeating three times, generate them with a macro.
///
/// Each builder must provide a method:
/// ```ignore
///   unsafe fn push_owned_impl(
///       &mut self,
///       py: pyo3::Python<'_>,
///       item: *mut pyo3::ffi::PyObject,   // caller hands over ownership
///   ) -> pyo3::PyResult<()>
/// ```
macro_rules! impl_push_methods {
    ($ty:ident) => {
        impl $ty {
            /// Push a scalar [`PyPickleVal`] (or anything that converts `Into<PyPickleVal>`).
            ///
            /// ```rust,ignore
            /// builder.push(py, 42isize)?
            ///        .push(py, "hello")?
            ///        .push(py, 3.14f64)?;
            /// ```
            pub fn push<'a, V>(&mut self, py: pyo3::Python<'_>, val: V) -> pyo3::PyResult<&mut Self>
            where
                V: Into<PyPickleVal<'a>>,
            {
                let raw = unsafe { val.into().into_py_raw(py)? };
                unsafe {
                    self.push_owned_impl(py, raw)?;
                }
                Ok(self)
            }

            /// Push a nested tuple whose items are filled by the closure `f`.
            ///
            /// `size` must equal the exact number of items `f` will push.
            ///
            /// ```rust,ignore
            /// builder.push_tuple(py, 2, |t| {
            ///     t.push(py, 3isize)?.push(py, 4isize)?;
            ///     Ok(())
            /// })?;
            /// ```
            pub fn push_tuple<F>(
                &mut self,
                py: pyo3::Python<'_>,
                size: isize,
                f: F,
            ) -> pyo3::PyResult<&mut Self>
            where
                F: FnOnce(&mut TupleBuilder) -> pyo3::PyResult<()>,
            {
                let mut b = TupleBuilder::new(py, size)?;
                f(&mut b)?;
                // into_raw transfers ownership; Drop becomes a no-op.
                unsafe {
                    self.push_owned_impl(py, b.into_raw())?;
                }
                Ok(self)
            }

            /// Push a nested list whose items are filled by the closure `f`.
            ///
            /// ```rust,ignore
            /// builder.push_list(py, |l| {
            ///     l.push(py, 1isize)?.push(py, "A")?;
            ///     Ok(())
            /// })?;
            /// ```
            pub fn push_list<F>(&mut self, py: pyo3::Python<'_>, f: F) -> pyo3::PyResult<&mut Self>
            where
                F: FnOnce(&mut ListBuilder) -> pyo3::PyResult<()>,
            {
                let mut b = ListBuilder::new(py)?;
                f(&mut b)?;
                unsafe {
                    self.push_owned_impl(py, b.into_raw())?;
                }
                Ok(self)
            }

            /// Push a nested dict whose entries are filled by the closure `f`.
            ///
            /// ```rust,ignore
            /// builder.push_dict(py, |d| {
            ///     d.entry(py, "key", 42isize)?;
            ///     Ok(())
            /// })?;
            /// ```
            pub fn push_dict<F>(&mut self, py: pyo3::Python<'_>, f: F) -> pyo3::PyResult<&mut Self>
            where
                F: FnOnce(&mut DictBuilder) -> pyo3::PyResult<()>,
            {
                let mut b = DictBuilder::new(py)?;
                f(&mut b)?;
                unsafe {
                    self.push_owned_impl(py, b.into_raw())?;
                }
                Ok(self)
            }
        }
    };
}

/// Builds the top-level Python tuple that represents a pickle state.
///
/// All slots **must** be filled before calling [`finish`](PickleBuilder::finish).
/// In debug builds an assertion verifies this; the tuple is otherwise valid but
/// partially initialised (CPython represents unfilled slots as `NULL`).
///
/// If the builder is dropped before `finish` is called, the partially-built
/// tuple is correctly decreffed and all already-inserted items are released.
pub struct PickleBuilder {
    /// `None` only after `finish()` has transferred ownership.
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
    size: isize,
    current: isize,
}

// private methods
impl PickleBuilder {
    fn new(py: pyo3::Python<'_>, size: isize) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyTuple_New(size) };
        if raw.is_null() {
            Err(pyo3::PyErr::fetch(py))
        } else {
            Ok(Self {
                inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
                size,
                current: 0,
            })
        }
    }

    /// # Safety
    /// `PyTuple_SetItem` **steals** `item` on success and **decrefs** it on
    /// failure, so this function must not touch `item`'s refcount after the call.
    unsafe fn push_owned_impl(
        &mut self,
        py: pyo3::Python<'_>,
        item: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        debug_assert!(
            self.current < self.size,
            "PickleBuilder: pushed more items than `size`"
        );
        let ptr = self.inner.expect("PickleBuilder already consumed").as_ptr();
        if pyo3::ffi::PyTuple_SetItem(ptr, self.current, item) != 0 {
            // item was already decreffed by PyTuple_SetItem on failure
            return Err(pyo3::PyErr::fetch(py));
        }
        self.current += 1;
        Ok(())
    }
}

impl PickleBuilder {
    /// Finalise the builder into a [`Pickle`].
    pub fn finish(mut self, py: pyo3::Python<'_>) -> Pickle {
        debug_assert_eq!(
            self.current,
            self.size,
            "PickleBuilder::finish called with {} unfilled slot(s)",
            self.size - self.current,
        );

        // Take ownership
        // `.take()` makes Drop no-op
        let ptr = self
            .inner
            .take()
            .expect("PickleBuilder already consumed")
            .as_ptr();

        let bound = unsafe { pyo3::Bound::from_owned_ptr(py, ptr) };
        Pickle(bound.unbind())
    }
}

impl_push_methods!(PickleBuilder);

impl Drop for PickleBuilder {
    fn drop(&mut self) {
        // Releases the tuple and all items already inserted into it.
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

/// Builds a Python tuple for embedding inside another container.
pub struct TupleBuilder {
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
    size: isize,
    current: isize,
}

// private methods
impl TupleBuilder {
    /// Consume the builder and surrender ownership of the raw pointer to the
    /// caller (used internally to insert into a parent container).
    fn into_raw(mut self) -> *mut pyo3::ffi::PyObject {
        // Drop becomes a no-op because `inner` is now None.
        self.inner
            .take()
            .expect("TupleBuilder already consumed")
            .as_ptr()
    }

    unsafe fn push_owned_impl(
        &mut self,
        py: pyo3::Python<'_>,
        item: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        debug_assert!(
            self.current < self.size,
            "TupleBuilder: pushed more items than `size`"
        );
        let ptr = self.inner.expect("TupleBuilder already consumed").as_ptr();
        if pyo3::ffi::PyTuple_SetItem(ptr, self.current, item) != 0 {
            return Err(pyo3::PyErr::fetch(py));
        }
        self.current += 1;
        Ok(())
    }
}

impl TupleBuilder {
    /// Allocate a new tuple with `size` pre-allocated slots.
    pub fn new(py: pyo3::Python<'_>, size: isize) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyTuple_New(size) };
        if raw.is_null() {
            Err(pyo3::PyErr::fetch(py))
        } else {
            Ok(Self {
                inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
                size,
                current: 0,
            })
        }
    }
}

impl_push_methods!(TupleBuilder);

impl Drop for TupleBuilder {
    fn drop(&mut self) {
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

/// Builds a Python list of arbitrary length.
///
/// Unlike [`TupleBuilder`], no size is required upfront; items are appended
/// one by one via [`PyList_Append`].
pub struct ListBuilder {
    /// `None` only after `into_raw()` or `build()`.
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
}

// private methods
impl ListBuilder {
    fn into_raw(mut self) -> *mut pyo3::ffi::PyObject {
        self.inner
            .take()
            .expect("ListBuilder already consumed")
            .as_ptr()
    }

    /// # Safety
    /// `PyList_Append` does **not** steal `item`; it increments `item`'s refcount
    /// on success.  We therefore always decref our owned ref after the call,
    /// regardless of success or failure.
    unsafe fn push_owned_impl(
        &mut self,
        py: pyo3::Python<'_>,
        item: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        let ptr = self.inner.expect("ListBuilder already consumed").as_ptr();
        let result = pyo3::ffi::PyList_Append(ptr, item);
        pyo3::ffi::Py_DECREF(item); // release our owned ref in all cases
        if result != 0 {
            return Err(pyo3::PyErr::fetch(py));
        }
        Ok(())
    }
}

impl ListBuilder {
    /// Create a new, empty list.
    pub fn new(py: pyo3::Python<'_>) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyList_New(0) };
        if raw.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }
        Ok(Self {
            inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
        })
    }
}

impl_push_methods!(ListBuilder);

impl Drop for ListBuilder {
    fn drop(&mut self) {
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

/// Builds a Python dict.
///
/// Keys must be [`PyPickleVal`] scalars (integers, floats, bools, strings, `None`).
/// Values may be scalars **or** nested containers built via the `entry_tuple`,
/// `entry_list`, and `entry_dict` methods.
pub struct DictBuilder {
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
}

// private methods
impl DictBuilder {
    fn into_raw(mut self) -> *mut pyo3::ffi::PyObject {
        self.inner
            .take()
            .expect("DictBuilder already consumed")
            .as_ptr()
    }

    /// # Safety
    /// `PyDict_SetItem` does **not** steal either `key` or `val`.
    /// This helper takes ownership of both and decrefs them unconditionally.
    unsafe fn set_kv(
        &mut self,
        py: pyo3::Python<'_>,
        key: *mut pyo3::ffi::PyObject,
        val: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        let ptr = self.inner.expect("DictBuilder already consumed").as_ptr();
        let result = pyo3::ffi::PyDict_SetItem(ptr, key, val);
        // Always release our owned refs regardless of success/failure.
        pyo3::ffi::Py_DECREF(key);
        pyo3::ffi::Py_DECREF(val);
        if result != 0 {
            Err(pyo3::PyErr::fetch(py))
        } else {
            Ok(())
        }
    }
}

impl DictBuilder {
    /// Create a new, empty dict.
    pub fn new(py: pyo3::Python<'_>) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyDict_New() };
        if raw.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }
        Ok(Self {
            inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
        })
    }

    /// Insert `key → val` where both are [`PyPickleVal`] scalars.
    pub fn entry<'k, 'v, K, V>(
        &mut self,
        py: pyo3::Python<'_>,
        key: K,
        val: V,
    ) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        V: Into<PyPickleVal<'v>>,
    {
        unsafe {
            let kptr = key.into().into_py_raw(py)?;
            let vptr = match val.into().into_py_raw(py) {
                Ok(v) => v,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(kptr); // clean up key we already allocated
                    return Err(e);
                }
            };
            self.set_kv(py, kptr, vptr)?;
        }
        Ok(self)
    }

    /// Insert `key → (nested tuple)`.
    pub fn entry_tuple<'k, K, F>(
        &mut self,
        py: pyo3::Python<'_>,
        key: K,
        size: isize,
        f: F,
    ) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        F: FnOnce(&mut TupleBuilder) -> pyo3::PyResult<()>,
    {
        let mut b = TupleBuilder::new(py, size)?;
        f(&mut b)?;
        let vptr = b.into_raw(); // transfer ownership out of TupleBuilder
        unsafe {
            let kptr = match key.into().into_py_raw(py) {
                Ok(k) => k,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(vptr); // release value we built
                    return Err(e);
                }
            };
            self.set_kv(py, kptr, vptr)?;
        }
        Ok(self)
    }

    /// Insert `key → [nested list]`.
    pub fn entry_list<'k, K, F>(
        &mut self,
        py: pyo3::Python<'_>,
        key: K,
        f: F,
    ) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        F: FnOnce(&mut ListBuilder) -> pyo3::PyResult<()>,
    {
        let mut b = ListBuilder::new(py)?;
        f(&mut b)?;
        let vptr = b.into_raw();
        unsafe {
            let kptr = match key.into().into_py_raw(py) {
                Ok(k) => k,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(vptr);
                    return Err(e);
                }
            };
            self.set_kv(py, kptr, vptr)?;
        }
        Ok(self)
    }

    /// Insert `key → {nested dict}`.
    pub fn entry_dict<'k, K, F>(
        &mut self,
        py: pyo3::Python<'_>,
        key: K,
        f: F,
    ) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        F: FnOnce(&mut DictBuilder) -> pyo3::PyResult<()>,
    {
        let mut b = DictBuilder::new(py)?;
        f(&mut b)?;
        let vptr = b.into_raw();
        unsafe {
            let kptr = match key.into().into_py_raw(py) {
                Ok(k) => k,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(vptr);
                    return Err(e);
                }
            };
            self.set_kv(py, kptr, vptr)?;
        }
        Ok(self)
    }
}

impl Drop for DictBuilder {
    fn drop(&mut self) {
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}
