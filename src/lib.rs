use pyo3::prelude::*;

mod cache;
mod base;

#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "awolverp")?;

    m.add_class::<base::BaseCacheImpl>()?;
    m.add_class::<cache::Cache>()?;

    Ok(())
}
