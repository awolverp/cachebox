use pyo3::prelude::*;

mod lazyheap;
mod linked_list;
mod mutex;

#[macro_use]
mod common;

mod bridge;
mod policies;

/// cachebox core ( written in Rust )
#[pymodule(gil_used = false)]
#[cold]
fn _core(py: pyo3::Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__author__", env!("CARGO_PKG_AUTHORS"))?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    m.add("CoreKeyError", py.get_type::<bridge::CoreKeyError>())?;

    m.add_class::<bridge::cache::Cache>()?;
    m.add_class::<bridge::fifocache::FIFOCache>()?;
    m.add_class::<bridge::rrcache::RRCache>()?;
    m.add_class::<bridge::lrucache::LRUCache>()?;
    m.add_class::<bridge::lfucache::LFUCache>()?;
    m.add_class::<bridge::ttlcache::TTLCache>()?;
    m.add_class::<bridge::vttlcache::VTTLCache>()?;
    m.add_class::<bridge::TTLPair>()?;
    m.add_class::<bridge::BaseCacheImpl>()?;

    Ok(())
}
