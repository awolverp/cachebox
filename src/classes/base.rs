use pyo3::prelude::*;

pub const ISIZE_MEMORY_SIZE: usize = std::mem::size_of::<isize>();
pub const PYOBJECT_MEMORY_SIZE: usize = std::mem::size_of::<Py<PyAny>>();

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

#[pyclass(name="_vec_one_value_iterator", module = "cachebox._cachebox")]
pub struct VecOneValueIterator {
    pub view: std::vec::IntoIter<Py<PyAny>>
}

#[pymethods]
impl VecOneValueIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Py<PyAny>> {
        slf.view.next()
    }
}

#[pyclass(name="_vec_items_iterator", module = "cachebox._cachebox")]
pub struct VecItemsIterator {
    pub view: std::vec::IntoIter<(Py<PyAny>, Py<PyAny>)>
}

#[pymethods]
impl VecItemsIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(Py<PyAny>, Py<PyAny>)> {
        slf.view.next()
    }
}

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! pyany_to_hash {
        ($key:expr, $py:expr) => {{
            let _ref = $key.as_ref($py);
            _ref.hash()
        }};
    }
}
