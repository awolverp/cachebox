use pyo3::prelude::*;

use crate::create_pyerr;

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
}
