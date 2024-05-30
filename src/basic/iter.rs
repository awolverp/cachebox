#![allow(non_camel_case_types)]

use super::HashablePyObject;
use crate::create_pyerr;
use pyo3::prelude::*;

pub struct SafeRawIter<I> {
    ptr: core::ptr::NonNull<pyo3::ffi::PyObject>,
    len: usize,
    iter: parking_lot::Mutex<hashbrown::raw::RawIter<I>>,
}

impl<I> SafeRawIter<I> {
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        len: usize,
        iter: hashbrown::raw::RawIter<I>,
    ) -> Self {
        unsafe {
            pyo3::ffi::Py_INCREF(ptr);
        }

        Self {
            ptr: unsafe { core::ptr::NonNull::new_unchecked(ptr) },
            len,
            iter: parking_lot::Mutex::new(iter),
        }
    }

    pub fn next(&mut self) -> PyResult<&I> {
        // SAFETY: we do not need to check error because we sure about implmenetation of the type
        if self.len != unsafe { pyo3::ffi::PyObject_Length(self.ptr.as_ptr()) as usize } {
            return Err(create_pyerr!(
                pyo3::exceptions::PyRuntimeError,
                "cache changed size during iteration"
            ));
        }

        let mut l = self.iter.lock();
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
pub struct items_iterator {
    pub safeiter: SafeRawIter<(HashablePyObject, PyObject)>,
}

#[pymethods]
impl items_iterator {
    pub fn size(&self) -> usize {
        self.safeiter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<(PyObject, PyObject)> {
        let (k, v) = slf.safeiter.next()?;
        Ok((k.object.clone(), v.clone()))
    }
}

#[pyclass(module = "cachebox._cachebox")]
pub struct keys_iterator {
    pub safeiter: SafeRawIter<(HashablePyObject, PyObject)>,
}

#[pymethods]
impl keys_iterator {
    pub fn size(&self) -> usize {
        self.safeiter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyObject> {
        let (k, _) = slf.safeiter.next()?;
        Ok(k.object.clone())
    }
}

#[pyclass(module = "cachebox._cachebox")]
pub struct values_iterator {
    pub safeiter: SafeRawIter<(HashablePyObject, PyObject)>,
}

#[pymethods]
impl values_iterator {
    pub fn size(&self) -> usize {
        self.safeiter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyObject> {
        let (_, v) = slf.safeiter.next()?;
        Ok(v.clone())
    }
}
