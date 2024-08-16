use pyo3::prelude::*;

#[macro_use]
mod util;
mod bridge;
mod hashedkey;
mod internal;
mod linked_list;
mod mutex;
mod sorted_heap;

const PYOBJECT_SIZE: usize = core::mem::size_of::<pyo3::PyObject>();
const HASHEDKEY_SIZE: usize = core::mem::size_of::<hashedkey::HashedKey>();

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
    m.add("__version__", CACHEBOX_VERSION)?;
    m.add("version_info", version_info())?;
    m.add("__author__", "awolverp")?;

    m.add_class::<bridge::baseimpl::BaseCacheImpl>()?;
    m.add_class::<bridge::cache::Cache>()?;
    m.add_class::<bridge::cache::cache_iterator>()?;
    m.add_class::<bridge::fifocache::FIFOCache>()?;
    m.add_class::<bridge::fifocache::fifocache_iterator>()?;
    m.add_class::<bridge::rrcache::RRCache>()?;
    m.add_class::<bridge::ttlcache::TTLCache>()?;
    m.add_class::<bridge::ttlcache::ttlcache_iterator>()?;
    m.add_class::<bridge::lrucache::LRUCache>()?;
    m.add_class::<bridge::lrucache::lrucache_iterator>()?;
    m.add_class::<bridge::lfucache::LFUCache>()?;
    m.add_class::<bridge::lfucache::lfucache_iterator>()?;
    m.add_class::<bridge::vttlcache::VTTLCache>()?;
    m.add_class::<bridge::vttlcache::vttlcache_iterator>()?;

    Ok(())
}
