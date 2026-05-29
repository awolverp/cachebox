use std::ptr;

use pyo3::IntoPyObject;

use crate::internal::alias;

pub enum PyPickleVal<'a> {
    Owned(alias::PyObject),
    Borrowed(&'a alias::PyObject),
    Str(&'a str),
    UnsignedBig(u128),
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
impl From<u128> for PyPickleVal<'static> {
    #[inline]
    fn from(v: u128) -> Self {
        PyPickleVal::UnsignedBig(v)
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
    unsafe fn into_raw(self, py: pyo3::Python<'_>) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let ptr = match self {
            Self::Owned(v) => v.into_ptr(),
            Self::Borrowed(v) => {
                let ptr = v.as_ptr();
                pyo3::ffi::Py_INCREF(ptr);
                ptr
            }
            Self::UnsignedBig(v) => v.into_pyobject(py)?.into_ptr(),
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
    pub fn builder<'py>(py: pyo3::Python<'py>, size: usize) -> pyo3::PyResult<PickleBuilder<'py>> {
        PickleBuilder::new(py, size)
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

mod sealed {
    /// Accepts a single raw owned pointer from a finished child builder.
    pub trait Receive {
        /// # Safety
        /// `item` must have refcount == 1; ownership is fully transferred.
        unsafe fn receive(&mut self, item: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<()>;
    }
}

pub trait Builder: Sized + sealed::Receive {
    fn py(&self) -> pyo3::Python<'_>;

    fn push<'a, V: Into<PyPickleVal<'a>>>(&mut self, val: V) -> pyo3::PyResult<&mut Self> {
        let raw = unsafe { val.into().into_raw(self.py())? };
        unsafe {
            self.receive(raw)?;
        }

        Ok(self)
    }

    fn begin_tuple<'a>(&'a mut self, size: usize) -> pyo3::PyResult<TupleBuilder<'a, Self>> {
        TupleBuilder::new(self, size)
    }

    fn begin_list<'a>(&'a mut self) -> pyo3::PyResult<ListBuilder<'a, Self>> {
        ListBuilder::new(self)
    }

    fn begin_dict<'a>(&'a mut self) -> pyo3::PyResult<DictBuilder<'a, Self>> {
        DictBuilder::new(self)
    }
}

/// Builds the top-level Python tuple that represents a pickle state.
///
/// All slots **must** be filled before calling [`finish`](PickleBuilder::finish).
/// In debug builds an assertion verifies this; the tuple is otherwise valid but
/// partially initialised (CPython represents unfilled slots as `NULL`).
///
/// If the builder is dropped before `finish` is called, the partially-built
/// tuple is correctly decreffed and all already-inserted items are released.
pub struct PickleBuilder<'py> {
    py: pyo3::Python<'py>,
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
    size: isize,
    current: isize,
}

impl<'py> PickleBuilder<'py> {
    fn new(py: pyo3::Python<'py>, size: usize) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyTuple_New(size as isize) };
        if raw.is_null() {
            return Err(pyo3::PyErr::fetch(py));
        }
        Ok(Self {
            py,
            inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
            size: size as isize,
            current: 0,
        })
    }

    pub fn finish(mut self) -> Pickle {
        debug_assert_eq!(
            self.current,
            self.size,
            "PickleBuilder::finish: {} unfilled slot(s)",
            self.size - self.current
        );
        let ptr = self.inner.take().expect("already consumed").as_ptr();
        Pickle(unsafe { pyo3::Bound::from_owned_ptr(self.py, ptr) }.unbind())
    }
}

impl sealed::Receive for PickleBuilder<'_> {
    unsafe fn receive(&mut self, item: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<()> {
        debug_assert!(
            self.current < self.size,
            "PickleBuilder: pushed more items than `size`"
        );
        let ptr = self.inner.expect("PickleBuilder already consumed").as_ptr();
        if pyo3::ffi::PyTuple_SetItem(ptr, self.current, item) != 0 {
            // item was already decreffed by PyTuple_SetItem on failure
            return Err(pyo3::PyErr::fetch(self.py));
        }
        self.current += 1;
        Ok(())
    }
}

impl<'py> Builder for PickleBuilder<'py> {
    #[inline]
    fn py(&self) -> pyo3::Python<'py> {
        self.py
    }
}

impl Drop for PickleBuilder<'_> {
    fn drop(&mut self) {
        // Releases the tuple and all items already inserted into it.
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

pub struct TupleBuilder<'a, P: Builder> {
    parent: &'a mut P,
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
    size: isize,
    current: isize,
}

impl<'a, P: Builder> TupleBuilder<'a, P> {
    fn new(parent: &'a mut P, size: usize) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyTuple_New(size as isize) };
        if raw.is_null() {
            return Err(pyo3::PyErr::fetch(parent.py()));
        }

        Ok(Self {
            parent,
            inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
            size: size as isize,
            current: 0,
        })
    }

    #[inline]
    pub fn end(mut self) -> pyo3::PyResult<()> {
        debug_assert_eq!(
            self.current,
            self.size,
            "TupleBuilder::end: {} unfilled slot(s)",
            self.size - self.current
        );
        let item = self.inner.take().expect("already consumed").as_ptr();
        unsafe {
            self.parent.receive(item)?;
        }
        Ok(())
    }
}

impl<P: Builder> sealed::Receive for TupleBuilder<'_, P> {
    unsafe fn receive(&mut self, item: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<()> {
        debug_assert!(self.current < self.size, "TupleBuilder: too many items");
        if pyo3::ffi::PyTuple_SetItem(
            self.inner.expect("already consumed").as_ptr(),
            self.current,
            item,
        ) != 0
        {
            return Err(pyo3::PyErr::fetch(self.parent.py()));
        }
        self.current += 1;
        Ok(())
    }
}

impl<P: Builder> Builder for TupleBuilder<'_, P> {
    #[inline]
    fn py(&self) -> pyo3::Python<'_> {
        self.parent.py()
    }
}

impl<P: Builder> Drop for TupleBuilder<'_, P> {
    fn drop(&mut self) {
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

pub struct ListBuilder<'a, P: Builder> {
    parent: &'a mut P,
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
}

impl<'a, P: Builder> ListBuilder<'a, P> {
    fn new(parent: &'a mut P) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyList_New(0) };
        if raw.is_null() {
            return Err(pyo3::PyErr::fetch(parent.py()));
        }
        Ok(Self {
            parent,
            inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
        })
    }

    #[inline]
    pub fn end(mut self) -> pyo3::PyResult<()> {
        let item = self.inner.take().expect("already consumed").as_ptr();
        unsafe {
            self.parent.receive(item)?;
        }
        Ok(())
    }
}

impl<P: Builder> sealed::Receive for ListBuilder<'_, P> {
    unsafe fn receive(&mut self, item: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<()> {
        let rc = pyo3::ffi::PyList_Append(self.inner.expect("already consumed").as_ptr(), item);
        pyo3::ffi::Py_DECREF(item); // PyList_Append does not steal
        if rc != 0 {
            Err(pyo3::PyErr::fetch(self.parent.py()))
        } else {
            Ok(())
        }
    }
}

impl<P: Builder> Builder for ListBuilder<'_, P> {
    #[inline]
    fn py(&self) -> pyo3::Python<'_> {
        self.parent.py()
    }
}

impl<P: Builder> Drop for ListBuilder<'_, P> {
    fn drop(&mut self) {
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

pub struct DictBuilder<'a, P: Builder> {
    parent: &'a mut P,
    inner: Option<ptr::NonNull<pyo3::ffi::PyObject>>,
}

impl<'a, P: Builder> DictBuilder<'a, P> {
    fn new(parent: &'a mut P) -> pyo3::PyResult<Self> {
        let raw = unsafe { pyo3::ffi::PyDict_New() };
        if raw.is_null() {
            return Err(pyo3::PyErr::fetch(parent.py()));
        }
        Ok(Self {
            parent,
            inner: Some(unsafe { ptr::NonNull::new_unchecked(raw) }),
        })
    }

    pub fn entry<'k, 'v, K, V>(&mut self, key: K, val: V) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        V: Into<PyPickleVal<'v>>,
    {
        let kptr = unsafe { key.into().into_raw(self.parent.py())? };
        let vptr = unsafe {
            match val.into().into_raw(self.parent.py()) {
                Ok(v) => v,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(kptr);
                    return Err(e);
                }
            }
        };
        unsafe {
            self.set_kv(kptr, vptr)?;
        }
        Ok(self)
    }

    #[inline]
    pub fn end(mut self) -> pyo3::PyResult<()> {
        let item = self.inner.take().expect("already consumed").as_ptr();
        unsafe {
            self.parent.receive(item)?;
        }
        Ok(())
    }

    unsafe fn set_kv(
        &mut self,
        key: *mut pyo3::ffi::PyObject,
        val: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        let rc =
            pyo3::ffi::PyDict_SetItem(self.inner.expect("already consumed").as_ptr(), key, val);
        pyo3::ffi::Py_DECREF(key);
        pyo3::ffi::Py_DECREF(val);
        if rc != 0 {
            Err(pyo3::PyErr::fetch(self.parent.py()))
        } else {
            Ok(())
        }
    }
}

// DictBuilder also implements Builder so that begin_tuple/list/dict work
// as value-builders inside a dict value context.
impl<P: Builder> sealed::Receive for DictBuilder<'_, P> {
    #[inline]
    unsafe fn receive(&mut self, item: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<()> {
        pyo3::ffi::Py_DECREF(item);
        Err(pyo3::exceptions::PyTypeError::new_err(
            "use entry() or entry_*() to insert into a DictBuilder",
        ))
    }
}

impl<P: Builder> Builder for DictBuilder<'_, P> {
    #[inline]
    fn py(&self) -> pyo3::Python<'_> {
        self.parent.py()
    }
}

impl<P: Builder> Drop for DictBuilder<'_, P> {
    fn drop(&mut self) {
        if let Some(nn) = self.inner.take() {
            unsafe {
                pyo3::ffi::Py_DECREF(nn.as_ptr());
            }
        }
    }
}

impl<'a, P: Builder> DictBuilder<'a, P> {
    pub fn entry_tuple<'k, K, F>(&mut self, key: K, size: usize, f: F) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        F: FnOnce(&mut TupleBuilder<Sink>) -> pyo3::PyResult<()>,
    {
        let mut sink = Sink(
            // SAFETY: the GIL is held for the entire lifetime of this builder because
            // the root PickleBuilder<'py> (which does own the 'py borrow) is kept alive
            // as our `parent`.
            unsafe { std::mem::transmute(self.parent.py()) },
        );

        let vptr = {
            let mut b = TupleBuilder::new(&mut sink, size)?;
            f(&mut b)?;
            b.inner.take().expect("already consumed").as_ptr()
        };

        let kptr = unsafe {
            match key.into().into_raw(self.parent.py()) {
                Ok(k) => k,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(vptr);
                    return Err(e);
                }
            }
        };

        unsafe {
            self.set_kv(kptr, vptr)?;
        }
        Ok(self)
    }

    pub fn entry_list<'k, K, F>(&mut self, key: K, f: F) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        F: FnOnce(&mut ListBuilder<Sink>) -> pyo3::PyResult<()>,
    {
        let mut sink = Sink(
            // SAFETY: the GIL is held for the entire lifetime of this builder because
            // the root PickleBuilder<'py> (which does own the 'py borrow) is kept alive
            // as our `parent`.
            unsafe { std::mem::transmute(self.parent.py()) },
        );

        let vptr = {
            let mut b = ListBuilder::new(&mut sink)?;
            f(&mut b)?;
            b.inner.take().expect("already consumed").as_ptr()
        };
        let kptr = unsafe {
            match key.into().into_raw(self.parent.py()) {
                Ok(k) => k,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(vptr);
                    return Err(e);
                }
            }
        };
        unsafe {
            self.set_kv(kptr, vptr)?;
        }
        Ok(self)
    }

    pub fn entry_dict<'k, K, F>(&mut self, key: K, f: F) -> pyo3::PyResult<&mut Self>
    where
        K: Into<PyPickleVal<'k>>,
        F: FnOnce(&mut DictBuilder<Sink>) -> pyo3::PyResult<()>,
    {
        let mut sink = Sink(
            // SAFETY: the GIL is held for the entire lifetime of this builder because
            // the root PickleBuilder<'py> (which does own the 'py borrow) is kept alive
            // as our `parent`.
            unsafe { std::mem::transmute(self.parent.py()) },
        );

        let vptr = {
            let mut b = DictBuilder::new(&mut sink)?;
            f(&mut b)?;
            b.inner.take().expect("already consumed").as_ptr()
        };
        let kptr = unsafe {
            match key.into().into_raw(self.parent.py()) {
                Ok(k) => k,
                Err(e) => {
                    pyo3::ffi::Py_DECREF(vptr);
                    return Err(e);
                }
            }
        };
        unsafe {
            self.set_kv(kptr, vptr)?;
        }
        Ok(self)
    }
}

/// A parent that simply discards the pointer it receives.
/// Used only inside `entry_*` closures where the container
/// extracts the raw pointer directly before `end()` is called.
pub struct Sink(pyo3::Python<'static>);

impl sealed::Receive for Sink {
    unsafe fn receive(&mut self, item: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<()> {
        pyo3::ffi::Py_DECREF(item);
        Ok(())
    }
}

impl Builder for Sink {
    #[inline]
    fn py(&self) -> pyo3::Python<'_> {
        self.0
    }
}
