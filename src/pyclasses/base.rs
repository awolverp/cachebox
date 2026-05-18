use crate::internal::alias;

crate::implement_pyclass! {
    /// Base implementation for cache classes in the cachebox library.
    ///
    /// This abstract base class defines the generic structure for cache implementations,
    /// supporting different key and value types through generic type parameters.
    /// Serves as a foundation for specific cache variants like Cache and FIFOCache.
    #[derive(Debug, Default, Clone, Copy)]
    [subclass, generic, frozen] PyBaseCacheImpl as "BaseCacheImpl" ;
}
crate::implement_pyclass! {
    /// Base implementation for cache classes in the cachebox library.
    ///
    /// This abstract base class defines the generic structure for cache implementations,
    /// supporting different key and value types through generic type parameters.
    /// Serves as a foundation for specific cache variants like Cache and FIFOCache.
    #[derive(Debug, Default, Clone, Copy)]
    [subclass, generic, frozen] PyAsyncBaseCacheImpl as "AsyncBaseCacheImpl" ;
}
crate::implement_pyclass! {
    /// Base implementation for cache classes in the cachebox library.
    ///
    /// This abstract base class defines the generic structure for cache implementations,
    /// supporting different key and value types through generic type parameters.
    /// Serves as a foundation for specific cache variants like Cache and FIFOCache.
    #[derive(Debug, Default, Clone, Copy)]
    [subclass, generic, frozen] PyBaseIteratorImpl as "BaseIteratorImpl" ;
}
crate::implement_pyclass! {
    /// Base implementation for cache classes in the cachebox library.
    ///
    /// This abstract base class defines the generic structure for cache implementations,
    /// supporting different key and value types through generic type parameters.
    /// Serves as a foundation for specific cache variants like Cache and FIFOCache.
    #[derive(Debug, Default, Clone, Copy)]
    [subclass, generic, frozen] PyAsyncBaseIteratorImpl as "AsyncBaseIteratorImpl" ;
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

#[pyo3::pymethods]
impl PyAsyncBaseCacheImpl {
    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    #[allow(unused_variables)]
    fn __new__(args: alias::ArgsType, kwargs: Option<alias::KwdsType>) -> Self {
        Self
    }

    fn __init__(&self) {}
}
