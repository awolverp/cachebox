#[allow(unused_imports)]
use pyo3::IntoPyObject;

macro_rules! err {
    ($type:ty, $val:expr) => {
        ::pyo3::PyErr::new::<$type, _>($val)
    };
}

#[rustfmt::skip]
macro_rules! non_zero_or {
    ($num:expr, $_else:expr) => {
        unsafe {
            core::num::NonZeroUsize::new_unchecked(
                if $num == 0 { $_else } else { $num }
            )
        }
    };
}

macro_rules! new_table {
    ($capacity:expr) => {{
        if $capacity > 0 {
            hashbrown::raw::RawTable::try_with_capacity($capacity)
                .map_err(|_| err!(pyo3::exceptions::PyMemoryError, ()))
        } else {
            Ok(hashbrown::raw::RawTable::new())
        }
    }};
}

macro_rules! tuple {
    (
        $py:expr,
        $len:expr,
        $($index:expr => $value:expr,)+
    ) => {{
        let tuple = unsafe { pyo3::ffi::PyTuple_New($len) };
        if tuple.is_null() {
            Err(pyo3::PyErr::fetch($py))
        } else {
            unsafe {
                $(
                    pyo3::ffi::PyTuple_SetItem(tuple, $index, $value);
                )+
            }

            Ok(tuple)
        }
    }};

    (check $tuple:expr, size=$size:expr) => {{
        if unsafe { pyo3::ffi::PyTuple_CheckExact($tuple) } == 0 {
            Err(err!(pyo3::exceptions::PyTypeError, "expected tuple, but got another type"))
        } else if unsafe {pyo3::ffi::PyTuple_Size($tuple)} != $size {
            Err(err!(pyo3::exceptions::PyTypeError, "tuple size is invalid"))
        } else {
            Ok(())
        }
    }}
}

macro_rules! extract_pickle_tuple {
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

            if pyo3::ffi::PyDict_CheckExact(obj) != 1 && pyo3::ffi::PyList_CheckExact(obj) != 1 {
                return Err(err!(
                    pyo3::exceptions::PyTypeError,
                    "the iterable object is not an dict or list"
                ));
            }

            // Tuple returns borrowed references
            pyo3::PyObject::from_borrowed_ptr($py, obj)
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

macro_rules! pyobject_eq {
    ($arg1:expr, $arg2:expr) => {
        if $arg1.as_ptr() == $arg2.as_ptr() {
            true
        } else {
            #[allow(unused_unsafe)]
            unsafe {
                let cmp = pyo3::ffi::PyObject_RichCompare(
                    $arg1.as_ptr(),
                    $arg2.as_ptr(),
                    pyo3::ffi::Py_EQ,
                );

                if cmp.is_null() {
                    pyo3::ffi::PyErr_Clear();
                    false
                } else {
                    let boolean = pyo3::ffi::PyObject_IsTrue(cmp);
                    pyo3::ffi::Py_DECREF(cmp);

                    if boolean == -1 {
                        pyo3::ffi::PyErr_Clear();
                        false
                    } else {
                        boolean == 1
                    }
                }
            }
        }
    };
}

unsafe fn _get_capacity(
    py: pyo3::Python<'_>,
    ptr: *mut pyo3::ffi::PyObject,
) -> pyo3::PyResult<usize> {
    unsafe fn inner(
        py: pyo3::Python<'_>,
        ptr: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        cfg_if::cfg_if! {
            if #[cfg(all(Py_3_9, not(any(Py_LIMITED_API, PyPy, GraalPy))))] {
                let m_name: pyo3::Bound<'_, pyo3::types::PyString> = "capacity".into_pyobject(py)?;
                Ok(pyo3::ffi::PyObject_CallMethodNoArgs(ptr, m_name.as_ptr()))
            } else {
                let capacity_fn =
                    pyo3::ffi::PyObject_GetAttrString(ptr, pyo3::ffi::c_str!("capacity").as_ptr());

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

    let result = inner(py, ptr)?;

    if result.is_null() {
        return Err(pyo3::PyErr::take(py).unwrap_unchecked());
    }

    let c = pyo3::ffi::PyLong_AsSize_t(result);
    pyo3::ffi::Py_XDECREF(result);

    Ok(c)
}

pub struct _KeepForIter<I> {
    pub ptr: core::ptr::NonNull<pyo3::ffi::PyObject>,
    pub capacity: usize,
    pub len: usize,

    phantom: core::marker::PhantomData<I>,
}

impl<I> _KeepForIter<I> {
    pub fn new(ptr: *mut pyo3::ffi::PyObject, capacity: usize, len: usize) -> Self {
        unsafe {
            pyo3::ffi::Py_INCREF(ptr);
        }

        Self {
            #[cfg(debug_assertions)]
            ptr: core::ptr::NonNull::new(ptr).unwrap(),
            #[cfg(not(debug_assertions))]
            ptr: unsafe { core::ptr::NonNull::new(ptr).unwrap_unchecked() },
            capacity,
            len,
            phantom: core::marker::PhantomData,
        }
    }

    pub fn status(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<()> {
        let capacity = unsafe { _get_capacity(py, self.ptr.as_ptr())? };
        if capacity != self.capacity {
            return Err(err!(
                pyo3::exceptions::PyRuntimeError,
                "cache changed size during iteration"
            ));
        }

        let len = unsafe { pyo3::ffi::PyObject_Size(self.ptr.as_ptr()) as usize };
        if len != self.len {
            return Err(err!(
                pyo3::exceptions::PyRuntimeError,
                "cache changed size during iteration"
            ));
        }

        Ok(())
    }
}

impl<I> Drop for _KeepForIter<I> {
    fn drop(&mut self) {
        unsafe {
            pyo3::ffi::Py_DECREF(self.ptr.as_ptr());
        }
    }
}

unsafe impl<I> Send for _KeepForIter<I> {}
unsafe impl<I> Sync for _KeepForIter<I> {}

pub struct NoLifetimeSliceIter<T> {
    pub slice: *const T,
    pub index: usize,
    pub len: usize,
}

impl<T> Iterator for NoLifetimeSliceIter<T> {
    type Item = *const T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            None
        } else {
            let value = unsafe { self.slice.add(self.index) };
            self.index += 1;
            Some(value)
        }
    }
}
