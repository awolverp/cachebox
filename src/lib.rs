#![feature(allocator_api)]
#![feature(dropck_eyepatch)]
#![feature(likely_unlikely)]

#[macro_use]
mod macro_rules;
mod hashbrown;
mod typeref;

pub mod internal;
pub mod policies;
pub mod pyclasses;

// TODO: refactor pickle
// TODO: refactor deepcopy

#[pyo3::pymodule]
mod _core {
    #[allow(unused_imports)]
    use pyo3::types::PyModuleMethods;

    use crate::typeref;

    #[pymodule_export]
    use crate::pyclasses::base::PyBaseCacheImpl;
    #[pymodule_export]
    use crate::pyclasses::base::PyBaseIteratorImpl;

    #[pymodule_export]
    use crate::pyclasses::cache::PyCache;
    #[pymodule_export]
    use crate::pyclasses::cache::PyCacheItems;
    #[pymodule_export]
    use crate::pyclasses::cache::PyCacheKeys;
    #[pymodule_export]
    use crate::pyclasses::cache::PyCacheValues;

    #[pymodule_export]
    use crate::pyclasses::fifocache::PyFIFOCache;
    #[pymodule_export]
    use crate::pyclasses::fifocache::PyFIFOCacheItems;
    #[pymodule_export]
    use crate::pyclasses::fifocache::PyFIFOCacheKeys;
    #[pymodule_export]
    use crate::pyclasses::fifocache::PyFIFOCacheValues;

    #[pymodule_export]
    use crate::pyclasses::rrcache::PyRRCache;
    #[pymodule_export]
    use crate::pyclasses::rrcache::PyRRCacheItems;
    #[pymodule_export]
    use crate::pyclasses::rrcache::PyRRCacheKeys;
    #[pymodule_export]
    use crate::pyclasses::rrcache::PyRRCacheValues;

    #[pymodule_export]
    use crate::pyclasses::lrucache::PyLRUCache;
    #[pymodule_export]
    use crate::pyclasses::lrucache::PyLRUCacheItems;
    #[pymodule_export]
    use crate::pyclasses::lrucache::PyLRUCacheKeys;
    #[pymodule_export]
    use crate::pyclasses::lrucache::PyLRUCacheValues;

    #[pymodule_export]
    use crate::pyclasses::lfucache::PyLFUCache;
    #[pymodule_export]
    use crate::pyclasses::lfucache::PyLFUCacheItems;
    #[pymodule_export]
    use crate::pyclasses::lfucache::PyLFUCacheKeys;
    #[pymodule_export]
    use crate::pyclasses::lfucache::PyLFUCacheValues;

    #[pymodule_export]
    use crate::pyclasses::ttlcache::PyTTLCache;
    #[pymodule_export]
    use crate::pyclasses::ttlcache::PyTTLCacheItems;
    #[pymodule_export]
    use crate::pyclasses::ttlcache::PyTTLCacheKeys;
    #[pymodule_export]
    use crate::pyclasses::ttlcache::PyTTLCacheValues;

    #[pymodule_init]
    pub fn init(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {
        typeref::initialize_typeref(m.py());

        #[cfg(feature = "small-offset")]
        m.add("_fifocache_small_offset", true)?;

        Ok(())
    }
}
