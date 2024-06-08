#![allow(non_camel_case_types)]

use super::HashablePyObject;
use crate::create_pyerr;
use pyo3::prelude::*;

pub struct SafeRawIter<I> {
    ptr: core::ptr::NonNull<pyo3::ffi::PyObject>,
    capacity: usize,
    pub len: usize,
    raw: parking_lot::Mutex<hashbrown::raw::RawIter<I>>,
}

impl<I> SafeRawIter<I> {
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
        let cap_fn_name =
            std::ffi::CString::new("capacity").expect("cannot call std::ffi::CString::new");

        // call `capacity()` to check changes in cache
        let (capacity, length) = unsafe {
            let capacity_fn = pyo3::ffi::PyObject_GetAttrString(
                self.ptr.as_ptr(),
                cap_fn_name.as_ptr() as *const std::ffi::c_char,
            );
            if capacity_fn.is_null() {
                return Err(pyo3::PyErr::take(py).unwrap_unchecked());
            }

            let result = pyo3::ffi::PyObject_CallNoArgs(capacity_fn);
            if result.is_null() {
                return Err(pyo3::PyErr::take(py).unwrap_unchecked());
            }

            let c = pyo3::ffi::PyLong_AsSize_t(result);
            pyo3::ffi::Py_XDECREF(result);

            (c, pyo3::ffi::PyObject_Size(self.ptr.as_ptr()) as usize)
        };

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

impl<I> Drop for SafeRawIter<I> {
    fn drop(&mut self) {
        unsafe {
            pyo3::ffi::Py_DECREF(self.ptr.as_ptr());
        }
    }
}

unsafe impl<I> Send for SafeRawIter<I> {}
unsafe impl<I> Sync for SafeRawIter<I> {}

#[pyclass(module = "cachebox._cachebox")]
pub struct tuple_ptr_iterator {
    iter: SafeRawIter<(HashablePyObject, PyObject)>,
}

impl tuple_ptr_iterator {
    pub fn new(iter: SafeRawIter<(HashablePyObject, PyObject)>) -> Self {
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

#[pyclass(module = "cachebox._cachebox")]
pub struct object_ptr_iterator {
    iter: SafeRawIter<(HashablePyObject, PyObject)>,
    index: u8,
}

impl object_ptr_iterator {
    pub fn new(iter: SafeRawIter<(HashablePyObject, PyObject)>, index: u8) -> Self {
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
