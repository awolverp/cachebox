#![feature(allocator_api)]
#![feature(dropck_eyepatch)]
#![feature(likely_unlikely)]
#![feature(optimize_attribute)]

#[macro_use]
mod macro_rules;

pub mod hashbrown;
pub mod internal;
pub mod pyclasses;

#[pyo3::pymodule]
mod _core {
    // use crate::typeref;

    // #[pymodule_export]
    // use crate::pyclasses::base::{PyBaseCacheImpl, PyBaseIteratorImpl};

    // #[pymodule_export]
    // use crate::pyclasses::cache::{PyCache, PyCacheItems, PyCacheKeys, PyCacheValues};

    #[pymodule_init]
    pub fn init(_m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {
        // typeref::initialize_typeref(m.py());
        Ok(())
    }
}
