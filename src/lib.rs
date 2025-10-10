#![feature(optimize_attribute)]

mod lazyheap;
mod linked_list;

#[macro_use]
mod common;

mod bridge;
mod policies;

/// cachebox core ( written in Rust )
#[pyo3::pymodule(gil_used = false)]
mod _core {
    use pyo3::types::PyModuleMethods;

    #[pymodule_export]
    use super::bridge::TTLPair;

    #[pymodule_export]
    use super::bridge::BaseCacheImpl;

    #[pymodule_export]
    use super::bridge::cache::Cache;

    #[pymodule_export]
    use super::bridge::fifocache::FIFOCache;

    #[pymodule_export]
    use super::bridge::rrcache::RRCache;

    #[pymodule_export]
    use super::bridge::lrucache::LRUCache;

    #[pymodule_export]
    use super::bridge::lfucache::LFUCache;
    
    #[pymodule_export]
    use super::bridge::ttlcache::TTLCache;
    
    #[pymodule_export]
    use super::bridge::vttlcache::VTTLCache;

    #[pymodule_init]
    fn init(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {
        m.add("__author__", env!("CARGO_PKG_AUTHORS"))?;
        m.add("__version__", env!("CARGO_PKG_VERSION"))?;

        m.add("CoreKeyError", m.py().get_type::<super::bridge::CoreKeyError>())?;

        Ok(())
    }
}
