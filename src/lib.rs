use pyo3::prelude::*;

mod basic;
mod cache;
mod fifocache;
mod lfucache;
mod lrucache;
mod rrcache;
mod ttlcache;
mod vttlcache;

const CACHEBOX_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version_info() -> (u8, u8, u8, bool) {
    let mut t: (u8, u8, u8, bool) = (0, 0, 0, false);

    for (index, mut sub) in CACHEBOX_VERSION.splitn(3, '.').enumerate() {
        if index == 2 {
            // -alpha, -beta, ...
            if let Some(x) = sub.find('-') {
                t.3 = true;
                sub = &sub[..x];
            }
        }

        match index {
            0 => t.0 = sub.parse().unwrap(),
            1 => t.1 = sub.parse().unwrap(),
            2 => t.2 = sub.parse().unwrap(),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    t
}

/// cachebox core ( written in Rust )
#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // constants
    m.add("__version__", CACHEBOX_VERSION)?;
    m.add("version_info", version_info())?;
    m.add("__author__", "awolverp")?;

    // classes
    m.add_class::<crate::basic::BaseCacheImpl>()?;
    m.add_class::<crate::cache::Cache>()?;
    m.add_class::<crate::fifocache::FIFOCache>()?;
    m.add_class::<crate::lfucache::LFUCache>()?;
    m.add_class::<crate::rrcache::RRCache>()?;
    m.add_class::<crate::lrucache::LRUCache>()?;
    m.add_class::<crate::ttlcache::TTLCache>()?;
    m.add_class::<crate::vttlcache::VTTLCache>()?;

    // iterators
    m.add_class::<crate::basic::iter::tuple_ptr_iterator>()?;
    m.add_class::<crate::basic::iter::object_ptr_iterator>()?;
    m.add_class::<crate::lfucache::lfu_tuple_ptr_iterator>()?;
    m.add_class::<crate::lfucache::lfu_object_ptr_iterator>()?;
    m.add_class::<crate::ttlcache::ttl_tuple_ptr_iterator>()?;
    m.add_class::<crate::ttlcache::ttl_object_ptr_iterator>()?;
    m.add_class::<crate::vttlcache::vttl_tuple_ptr_iterator>()?;
    m.add_class::<crate::vttlcache::vttl_object_ptr_iterator>()?;

    Ok(())
}
