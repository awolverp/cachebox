#![allow(non_camel_case_types)]

use super::HashablePyObject;
use crate::create_pyerr;
use pyo3::prelude::*;

pub struct SafeRawIter<I> {
    ptr: core::ptr::NonNull<pyo3::ffi::PyObject>,
    pub len: usize,
    raw: parking_lot::Mutex<hashbrown::raw::RawIter<I>>,
}

impl<I> SafeRawIter<I> {
    pub fn new(ptr: *mut pyo3::ffi::PyObject, len: usize, raw: hashbrown::raw::RawIter<I>) -> Self {
        unsafe {
            pyo3::ffi::Py_INCREF(ptr);
        }

        Self {
            ptr: unsafe { core::ptr::NonNull::new_unchecked(ptr) },
            len,
            raw: parking_lot::Mutex::new(raw),
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
    pub fn size(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<(PyObject, PyObject)> {
        let (k, v) = slf.iter.next()?;
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
    pub fn size(&self) -> usize {
        self.iter.len
    }

    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyObject> {
        if slf.index == 0 {
            let (k, _) = slf.iter.next()?;
            Ok(k.object.clone())
        } else if slf.index == 1 {
            let (_, v) = slf.iter.next()?;
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
