use pyo3::prelude::*;

mod basic;
mod cache;
mod fifocache;
mod lfucache;

#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "awolverp")?;

    m.add_class::<crate::basic::BaseCacheImpl>()?;
    m.add_class::<crate::cache::Cache>()?;
    m.add_class::<crate::fifocache::FIFOCache>()?;
    m.add_class::<crate::lfucache::LFUCache>()?;

    Ok(())
}
