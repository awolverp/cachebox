pub trait PickleMethods: Sized {
    /// uses for __getstate__
    ///
    /// Must return `tuple`
    unsafe fn dumps(&self) -> *mut pyo3::ffi::PyObject;

    /// uses for __setstate__
    ///
    /// `state` is always a tuple
    unsafe fn loads(
        &mut self,
        state: *mut pyo3::ffi::PyObject,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<()>;
}

#[macro_export]
macro_rules! pickle_check_state {
    ($py:expr, $state:expr, $size:expr) => {{
        if !$state.bind($py).is_instance_of::<pyo3::types::PyTuple>() {
            Err($crate::create_pyerr!(
                pyo3::exceptions::PyTypeError,
                "expected tuple, but got another type"
            ))
        } else {
            let tuple = $state.as_ptr();
            if unsafe { pyo3::ffi::PyTuple_Size(tuple) != $size } {
                Err($crate::create_pyerr!(
                    pyo3::exceptions::PyTypeError,
                    "tuple length is invalid"
                ))
            } else {
                Ok(tuple)
            }
        }
    }};
}

#[macro_export]
macro_rules! pickle_get_first_objects {
    ($py:expr, $state:expr) => {{
        let maxsize = {
            let obj = pyo3::ffi::PyTuple_GetItem($state, 0);
            pyo3::ffi::PyLong_AsSize_t(obj)
        };

        if let Some(e) = pyo3::PyErr::take($py) {
            return Err(e);
        }

        let iterable = {
            let obj = pyo3::ffi::PyTuple_GetItem($state, 1);

            if pyo3::ffi::PyDict_CheckExact(obj) != 1 {
                return Err(create_pyerr!(
                    pyo3::exceptions::PyTypeError,
                    "the iterable object is not an dict"
                ));
            }

            // Tuple uses borrowed references
            PyObject::from_borrowed_ptr($py, obj)
        };

        let capacity = {
            let obj = pyo3::ffi::PyTuple_GetItem($state, 2);
            pyo3::ffi::PyLong_AsSize_t(obj)
        };

        if let Some(e) = pyo3::PyErr::take($py) {
            return Err(e);
        }

        (maxsize, iterable, capacity)
    }};
}
