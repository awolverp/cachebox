//! implement [`BaseCacheImpl`], the base class of all classes.

use pyo3::types::PyTypeMethods;

/// This is the base class of all cache classes such as Cache, FIFOCache, ...
///
/// Do not try to call its constructor, this is only for type-hint.
#[pyo3::pyclass(module = "cachebox._cachebox", subclass, frozen)]
pub struct BaseCacheImpl {}

#[pyo3::pymethods]
impl BaseCacheImpl {
    #[new]
    #[pyo3(signature = (*args, **kwargs))]
    #[classmethod]
    #[allow(unused_variables)]
    pub fn __new__(
        cls: &pyo3::Bound<'_, pyo3::types::PyType>,
        args: &pyo3::Bound<'_, pyo3::PyAny>,
        kwargs: Option<&pyo3::Bound<'_, pyo3::PyAny>>,
    ) -> pyo3::PyResult<Self> {
        let size = unsafe { pyo3::ffi::PyTuple_Size(cls.mro().as_ptr()) };

        // This means BaseCacheImpl is used as subclass
        // So we shouldn't raise NotImplementedError
        if size > 2 {
            Ok(Self {})
        } else {
            Err(err!(pyo3::exceptions::PyNotImplementedError, "do not call this constructor, you can subclass this implementation or use other classes."))
        }
    }

    #[allow(unused_variables)]
    #[classmethod]
    pub fn __class_getitem__(
        cls: &pyo3::Bound<'_, pyo3::types::PyType>,
        args: pyo3::PyObject,
    ) -> pyo3::PyObject {
        cls.clone().into()
    }
}
