use pyo3::prelude::*;

/// cachebox core ( written in Rust )
#[pymodule(gil_used = false)]
#[cold]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__author__", env!("CARGO_PKG_AUTHORS"))?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    Ok(())
}
