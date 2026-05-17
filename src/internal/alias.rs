/// Type alias for `pyo3::Py<pyo3::PyAny>`
pub type PyObject = pyo3::Py<pyo3::PyAny>;

/// Type alias for `pyo3::Bound<'a, pyo3::PyAny>`
pub type BoundObject<'a> = pyo3::Bound<'a, pyo3::PyAny>;

/// Type alias for `&'a pyo3::Bound<'a, pyo3::types::PyTuple>`.
/// Use it directly as `args` argument type.
pub type BoundArgs<'a> = &'a pyo3::Bound<'a, pyo3::types::PyTuple>;

/// Type alias for `&'a pyo3::Bound<'a, pyo3::types::PyDict>`.
/// Use it directly as `kwds` argument type.
pub type BoundKwargs<'a> = &'a pyo3::Bound<'a, pyo3::types::PyDict>;
