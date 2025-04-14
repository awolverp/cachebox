use pyo3::create_exception;

create_exception!(cachebox._core, CoreKeyError, pyo3::exceptions::PyException);

pub mod cache;
pub mod fifocache;
pub mod lfucache;
pub mod lrucache;
pub mod rrcache;
pub mod ttlcache;
