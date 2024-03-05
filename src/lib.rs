use pyo3::prelude::*;

// Internal implementions
mod internal;
mod classes;

#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(_py: Python, m: &PyModule) -> PyResult<()> {
    // Variables
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "aWolverP")?;

    // Classes
    m.add_class::<classes::base::BaseCacheImpl>()?;
    m.add_class::<classes::cache::Cache>()?;

    Ok(())
}
