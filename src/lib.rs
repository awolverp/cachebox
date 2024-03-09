use pyo3::prelude::*;

// Internal implementations
mod classes;
mod internal;

#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "aWolverP")?;
    m.add_class::<classes::BaseCacheImpl>()?;
    m.add_class::<classes::Cache>()?;
    m.add_class::<classes::FIFOCache>()?;
    m.add_class::<classes::LFUCache>()?;
    m.add_class::<classes::LRUCache>()?;
    m.add_class::<classes::RRCache>()?;
    m.add_class::<classes::TTLCache>()?;
    m.add_class::<classes::VTTLCache>()?;

    Ok(())
}
