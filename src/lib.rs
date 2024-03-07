use pyo3::prelude::*;

// Internal implementions
mod classes;
mod internal;

#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(_py: Python, m: &PyModule) -> PyResult<()> {
    // Variables
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "aWolverP")?;

    // Classes
    m.add_class::<classes::BaseCacheImpl>()?;
    m.add_class::<classes::Cache>()?;

    Ok(())
}
