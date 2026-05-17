use std::ptr;

use crate::internal::alias;

/// Pickle object
pub struct Pickle(
    // Always is tuple
    alias::PyObject,
);

pub struct PickleBuilder {
    // Always is tuple
    tuple: ptr::NonNull<pyo3::ffi::PyObject>,
    size: isize,
    current: isize,
}

impl Pickle {
    pub fn builder(py: pyo3::Python, size: isize) -> pyo3::PyResult<PickleBuilder> {
        let tuple = unsafe { pyo3::ffi::PyTuple_New(size) };

        if tuple.is_null() {
            Err(pyo3::PyErr::fetch(py))
        } else {
            Ok(PickleBuilder {
                tuple: unsafe { ptr::NonNull::new_unchecked(tuple) },
                size,
                current: 0,
            })
        }
    }
}

impl From<Pickle> for alias::PyObject {
    fn from(value: Pickle) -> Self {
        value.0
    }
}

impl PickleBuilder {
    pub fn unsigned(&mut self, val: usize) -> &mut Self {
        debug_assert!(self.current < self.size);

        unsafe {
            let x = pyo3::ffi::PyLong_FromSize_t(val);
            debug_assert!(!x.is_null());

            debug_assert!(pyo3::ffi::PyTuple_SetItem(self.tuple.as_ptr(), self.current, x) == 0);
        }

        self.current += 1;
        self
    }

    pub fn signed(&mut self, val: isize) -> &mut Self {
        debug_assert!(self.current < self.size);

        unsafe {
            let x = pyo3::ffi::PyLong_FromSsize_t(val);
            debug_assert!(!x.is_null());

            debug_assert!(pyo3::ffi::PyTuple_SetItem(self.tuple.as_ptr(), self.current, x) == 0);
        }

        self.current += 1;
        self
    }

    pub fn finish(self, py: pyo3::Python) -> Pickle {
        let bound = unsafe { pyo3::Bound::from_owned_ptr(py, self.tuple.as_ptr()) };
        Pickle(bound.unbind())
    }
}
