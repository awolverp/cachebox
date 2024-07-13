pub mod iter;
#[macro_use]
mod pickle;

use core::hash::{Hash, Hasher};
pub use pickle::PickleMethods;
use pyo3::prelude::*;

pub const PYOBJECT_MEM_SIZE: usize = core::mem::size_of::<pyo3::PyObject>();
pub const HASHABLE_PYOBJECT_MEM_SIZE: usize = core::mem::size_of::<HashablePyObject>();

#[macro_export]
macro_rules! create_pyerr {
    ($err:ty, $val:expr) => {
        pyo3::PyErr::new::<$err, _>($val)
    };

    ($err:ty) => {
        pyo3::PyErr::new::<$err, _>(())
    };
}

/// A base class for all cache algorithms;
/// Do not try to call its constructor, this is only for type-hint.
///
/// You can use it for type hint or use it for type checking.
#[pyclass(subclass, module = "cachebox._cachebox")]
pub struct BaseCacheImpl;

#[pymethods]
impl BaseCacheImpl {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0),)]
    #[allow(unused_variables)]
    pub fn new(maxsize: usize, capacity: usize) -> PyResult<Self> {
        Err(create_pyerr!(pyo3::exceptions::PyNotImplementedError))
    }

    #[allow(unused_variables)]
    #[pyo3(signature=(generics))]
    #[staticmethod]
    pub fn __class_getitem__(generics: PyObject) {}
}

#[cfg_attr(debug_assertions, derive(Debug))]
pub struct HashablePyObject {
    pub object: pyo3::PyObject,
    pub hash: u64,
}

macro_rules! hash_object {
    ($value:expr, $py:expr) => {{
        let hash = unsafe { pyo3::ffi::PyObject_Hash($value.as_ptr()) };

        if hash == -1 {
            return Err(pyo3::PyErr::fetch($py));
        }

        let mut state = ahash::AHasher::default();
        hash.hash(&mut state);
        state.finish()
    }};
}

#[macro_export]
macro_rules! make_eq_func {
    ($key:expr) => {
        |(x, _)| x.eq(&$key)
    };
}

#[macro_export]
macro_rules! make_hasher_func {
    () => {
        |(x, _)| x.hash
    };
}

impl HashablePyObject {
    #[inline]
    pub fn try_from_pyobject(value: pyo3::PyObject, py: pyo3::Python<'_>) -> pyo3::PyResult<Self> {
        let state = hash_object!(value, py);
        Ok(Self {
            object: value,
            hash: state,
        })
    }

    #[inline]
    pub fn try_from_bound(value: pyo3::Bound<'_, pyo3::PyAny>) -> pyo3::PyResult<Self> {
        let state = hash_object!(value, value.py());
        Ok(Self {
            object: value.unbind(),
            hash: state,
        })
    }

    #[inline]
    fn compare_eq(&self, obj: &HashablePyObject) -> bool {
        unsafe {
            let cmp = pyo3::ffi::PyObject_RichCompare(
                self.object.as_ptr(),
                obj.object.as_ptr(),
                pyo3::pyclass::CompareOp::Eq as std::os::raw::c_int,
            );

            if cmp.is_null() {
                pyo3::ffi::PyErr_Clear();
                return false;
            }

            let t = pyo3::ffi::PyObject_IsTrue(cmp);
            pyo3::ffi::Py_DECREF(cmp);

            t == 1
        }
    }
}

impl PartialEq for HashablePyObject {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.compare_eq(other)
    }
}

impl Eq for HashablePyObject {}

impl Clone for HashablePyObject {
    fn clone(&self) -> Self {
        Self {
            object: self.object.clone(),
            hash: self.hash,
        }
    }
}
