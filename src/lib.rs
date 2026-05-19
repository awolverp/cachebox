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

#[pyo3::pymodule]
mod _core {
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

    #[pymodule_init]
    pub fn init(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {
        typeref::initialize_typeref(m.py());
        Ok(())
    }
}
