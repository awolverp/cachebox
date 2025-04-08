use pyo3::prelude::*;

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

    Ok(())
}
