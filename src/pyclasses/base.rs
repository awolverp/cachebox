use crate::internal::alias;

crate::implement_pyclass! {
    /// Base implementation for cache classes.
    ///
    /// This abstract base class defines the generic structure for cache
    /// implementations.
    #[derive(Debug, Default, Clone, Copy)]
    [subclass, generic, frozen] PyBaseCacheImpl as "BaseCacheImpl" ;
}

#[pyo3::pymethods]
impl PyBaseCacheImpl {
    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    #[allow(unused_variables)]
    fn __new__(args: alias::ArgsType, kwargs: Option<alias::KwdsType>) -> Self {
        Self
    }

    fn __init__(&self) {}
}
