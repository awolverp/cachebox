use pyo3::prelude::*;

pub const ISIZE_MEMORY_SIZE: usize = std::mem::size_of::<isize>();

#[derive(Clone)]
pub struct KeyValuePair(pub Py<PyAny>, pub Py<PyAny>);

#[pyclass(subclass, module = "cachebox._cachebox")]
pub struct BaseCacheImpl {}

#[pymethods]
impl BaseCacheImpl {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0))]
    pub fn __new__(maxsize: usize, capacity: usize) -> PyResult<Self> {
        let _ = maxsize;
        let _ = capacity;
        Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "This type is not implemented; use other implementions.",
        ))
    }
}

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! pyany_to_hash {
        ($key:expr, $py:expr) => {
            {
                let _ref = $key.as_ref($py);
                _ref.hash()
            }
        };
    }

    #[macro_export]
    macro_rules! use_rwlock {
        (r $rwlock:expr) => {
            $rwlock.read().unwrap()
        };

        (w $rwlock:expr) => {
            $rwlock.write().unwrap()
        }
    }
}
