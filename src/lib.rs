use pyo3::prelude::*;

mod base;
mod cache;
mod fifo;
mod lfu;
mod rr;
mod lru;
mod mru;
mod ttl;

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "aWolverP")?;
    m.add("__doc__", "The fastest caching library written in Rust")?;
    m.add_class::<base::BaseCacheImpl>()?;
    m.add_class::<cache::Cache>()?;
    m.add_class::<fifo::FIFOCache>()?;
    m.add_class::<lfu::LFUCache>()?;
    m.add_class::<rr::RRCache>()?;
    m.add_class::<lru::LRUCache>()?;
    m.add_class::<mru::MRUCache>()?;
    m.add_class::<ttl::TTLCacheNoDefault>()?;
    m.add_class::<ttl::TTLCache>()?;
    Ok(())
}
