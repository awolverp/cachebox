use pyo3::create_exception;

create_exception!(_core, CoreKeyError, pyo3::exceptions::PyException);

pub mod cache;
pub mod fifocache;
