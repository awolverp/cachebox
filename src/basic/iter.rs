#![allow(non_camel_case_types)]

use super::HashablePyObject;
use crate::create_pyerr;
use pyo3::prelude::*;

/// Calls `capacity()` method
unsafe fn call_capacity_method(
    ptr: *mut pyo3::ffi::PyObject,
    py: Python<'_>,
) -> PyResult<*mut pyo3::ffi::PyObject> {
    let cap_fn_name =
        std::ffi::CString::new("capacity").expect("cannot call std::ffi::CString::new");

    cfg_if::cfg_if! {
        if #[cfg(all(Py_3_9, not(any(Py_LIMITED_API, PyPy, GraalPy))))] {
            Ok(pyo3::ffi::PyObject_CallMethodNoArgs(ptr, cap_fn_name.as_ptr() as *const std::ffi::c_char))
        } else {
            let capacity_fn =
                pyo3::ffi::PyObject_GetAttrString(ptr, cap_fn_name.as_ptr() as *const std::ffi::c_char);

            if capacity_fn.is_null() {
                return Err(pyo3::PyErr::take(py).unwrap_unchecked());
            }

            let empty_args = pyo3::ffi::PyTuple_New(0);
            let result = pyo3::ffi::PyObject_Call(capacity_fn, empty_args, std::ptr::null_mut());
            pyo3::ffi::Py_XDECREF(empty_args);
            pyo3::ffi::Py_XDECREF(capacity_fn);

            Ok(result)
        }
    }
}

/// Calls `capacity()` method and converts its result to `usize`
unsafe fn get_capacity(ptr: *mut pyo3::ffi::PyObject, py: Python<'_>) -> PyResult<usize> {
    let result = call_capacity_method(ptr, py)?;

    if result.is_null() {
        return Err(pyo3::PyErr::take(py).unwrap_unchecked());
    }

    let c = pyo3::ffi::PyLong_AsSize_t(result);
    pyo3::ffi::Py_XDECREF(result);

    Ok(c)
}

/// Iter around `hashbrown::raw::RawIter<I>` without worry!
pub struct SafeRawHashMapIter<I> {
    ptr: core::ptr::NonNull<pyo3::ffi::PyObject>,
    capacity: usize,
    pub len: usize,
    raw: parking_lot::Mutex<hashbrown::raw::RawIter<I>>,
}

impl<I> SafeRawHashMapIter<I> {
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        capacity: usize,
        len: usize,
        raw: hashbrown::raw::RawIter<I>,
    ) -> Self {
        unsafe {
            pyo3::ffi::Py_INCREF(ptr);
        }

        Self {
            ptr: unsafe { core::ptr::NonNull::new_unchecked(ptr) },
            capacity,
            len,
            raw: parking_lot::Mutex::new(raw),
        }
    }

    pub fn next(&mut self, py: Python<'_>) -> PyResult<&I> {
        let capacity = unsafe { get_capacity(self.ptr.as_ptr(), py)? };
        let length = unsafe { pyo3::ffi::PyObject_Size(self.ptr.as_ptr()) as usize };

        if (self.capacity != capacity) || (self.len != length) {
            return Err(create_pyerr!(
                pyo3::exceptions::PyRuntimeError,
                "cache changed size during iteration"
            ));
        }

        let mut l = self.raw.lock();
        if let Some(x) = l.next() {
            return Ok(unsafe { x.as_ref() });
        }

        Err(create_pyerr!(pyo3::exceptions::PyStopIteration))
    }
}

impl<I> Drop for SafeRawHashMapIter<I> {
    fn drop(&mut self) {
        unsafe {
            pyo3::ffi::Py_DECREF(self.ptr.as_ptr());
        }
    }
}

unsafe impl<I> Send for SafeRawHashMapIter<I> {}
unsafe impl<I> Sync for SafeRawHashMapIter<I> {}

/// Items iterator
#[pyclass(module = "cachebox._cachebox")]
pub struct tuple_ptr_iterator {
    iter: SafeRawHashMapIter<(HashablePyObject, PyObject)>,
}

impl tuple_ptr_iterator {
    pub fn new(iter: SafeRawHashMapIter<(HashablePyObject, PyObject)>) -> Self {
        Self { iter }
    }
}

#[pymethods]
impl tuple_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<(PyObject, PyObject)> {
        let (k, v) = slf.iter.next(py)?;
        Ok((k.object.clone(), v.clone()))
    }
}

/// Key or value iterator
#[pyclass(module = "cachebox._cachebox")]
pub struct object_ptr_iterator {
    iter: SafeRawHashMapIter<(HashablePyObject, PyObject)>,
    index: u8,
}

impl object_ptr_iterator {
    pub fn new(iter: SafeRawHashMapIter<(HashablePyObject, PyObject)>, index: u8) -> Self {
        Self { iter, index }
    }
}

#[pymethods]
impl object_ptr_iterator {
    pub fn __len__(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<PyObject> {
        if slf.index == 0 {
            let (k, _) = slf.iter.next(py)?;
            Ok(k.object.clone())
        } else if slf.index == 1 {
            let (_, v) = slf.iter.next(py)?;
            Ok(v.clone())
        } else {
            #[cfg(debug_assertions)]
            unreachable!("invalid iteration index specified");

            #[cfg(not(debug_assertions))]
            unsafe {
                core::hint::unreachable_unchecked();
            }
        }
    }
}
