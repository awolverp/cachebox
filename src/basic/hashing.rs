use core::hash::{Hash, Hasher};

#[derive(Clone)]
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
        let res = unsafe {
            pyo3::ffi::PyObject_RichCompareBool(
                self.object.as_ptr(),
                obj.object.as_ptr(),
                pyo3::pyclass::CompareOp::Eq as std::os::raw::c_int,
            )
        };

        if res == -1 {
            unsafe {
                pyo3::ffi::PyErr_Clear();
            }
        }

        res == 1
    }
}

impl PartialEq for HashablePyObject {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.compare_eq(other)
    }
}

impl Eq for HashablePyObject {}
