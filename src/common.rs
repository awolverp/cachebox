#[inline(always)]
pub fn pyobject_equal(
    py: pyo3::Python<'_>,
    arg1: *mut pyo3::ffi::PyObject,
    arg2: *mut pyo3::ffi::PyObject,
) -> pyo3::PyResult<bool> {
    unsafe {
        if std::ptr::eq(arg1, arg2) {
            return Ok(true);
        }

        let boolean = pyo3::ffi::PyObject_RichCompareBool(arg1, arg2, pyo3::ffi::Py_EQ);

        if boolean < 0 {
            Err(pyo3::PyErr::take(py).unwrap_unchecked())
        } else {
            Ok(boolean == 1)
        }
    }
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
                .map_err(|_| pyo3::PyErr::new::<pyo3::exceptions::PyMemoryError, _>(()))
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
        #[allow(unused_unsafe)]
        let tuple = unsafe { pyo3::ffi::PyTuple_New($len) };
        if tuple.is_null() {
            Err(pyo3::PyErr::fetch($py))
        } else {
            #[allow(unused_unsafe)]
            unsafe {
                $(
                    pyo3::ffi::PyTuple_SetItem(tuple, $index, $value);
                )+
            }

            Ok(tuple)
        }
    }};

    (check $tuple:expr, size=$size:expr) => {{
        #[allow(unused_unsafe)]
        if unsafe { pyo3::ffi::PyTuple_CheckExact($tuple) } == 0 {
            Err(
                pyo3::PyErr::new::<pyo3::exceptions::PyTypeError, _>("expected tuple, but got another type")
            )
        } else if unsafe {pyo3::ffi::PyTuple_Size($tuple)} != $size {
            Err(
                pyo3::PyErr::new::<pyo3::exceptions::PyTypeError, _>("tuple size is invalid")
            )
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
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "the iterable object is not an dict or list",
                ));
            }

            // Tuple returns borrowed reference
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

/// Converts an isize value to a u64 value, mapping negative values to the upper half of the u64 range.
///
/// This function ensures a bijective mapping between isize and u64, preserving the order of values
/// by offsetting negative values to the upper range of u64.
#[inline(always)]
fn convert_isize_to_u64(v: &isize) -> u64 {
    const OFFSET: u64 = 0x8000000000000000; // 1 << 63

    if *v >= 0 {
        *v as u64
    } else {
        (-(*v + 1)) as u64 + OFFSET
    }
}

/// Precomputed Hash PyObject
///
/// A precomputed hash is a cryptographic hash value that's calculated in advance
/// and stored for later use, rather than being computed on demand when needed.
pub struct PreHashObject {
    pub obj: pyo3::PyObject,
    pub hash: u64,
}

impl PreHashObject {
    /// Creates a new [`PreHashObject`]
    pub fn new(obj: pyo3::PyObject, hash: u64) -> Self {
        Self { obj, hash }
    }

    /// Calculates the hash of `object` and creates a new [`PreHashObject`]
    pub fn from_pyobject(py: pyo3::Python<'_>, object: pyo3::PyObject) -> pyo3::PyResult<Self> {
        unsafe {
            let py_hash = pyo3::ffi::PyObject_Hash(object.as_ptr());

            if py_hash == -1 {
                // SAFETY:
                // PyObject_Hash never returns -1 on success.
                return Err(pyo3::PyErr::take(py).unwrap_unchecked());
            }

            Ok(Self::new(object, convert_isize_to_u64(&py_hash)))
        }
    }

    /// Check equality of two objects by using [`pyo3::ffi::PyObject_RichCompareBool`]
    pub fn equal(&self, py: pyo3::Python<'_>, other: &Self) -> pyo3::PyResult<bool> {
        pyobject_equal(py, self.obj.as_ptr(), other.obj.as_ptr())
    }
}

impl std::fmt::Debug for PreHashObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PreHashObject({})", self.hash)
    }
}

/// A view into a single entry in a table, which may either be absent or occupied.
///
/// This is common in policies and will be used by `entry(...)` methods of them.
pub enum Entry<O, V> {
    Occupied(O),
    Absent(V),
}

/// A trait for adding `try_find` and `try_find_entry` methods to [`hashbrown::HashTable`]
pub trait TryFindMethods<T> {
    /// Searches for an element in the table.
    fn try_find<E>(
        &self,
        hash: u64,
        compare: impl FnMut(&T) -> Result<bool, E>,
    ) -> Result<Option<hashbrown::raw::Bucket<T>>, E>;

    fn try_find_or_find_insert_slot<E>(
        &mut self,
        hash: u64,
        compare: impl FnMut(&T) -> Result<bool, E>,
        hasher: impl Fn(&T) -> u64,
    ) -> Result<Result<hashbrown::raw::Bucket<T>, hashbrown::raw::InsertSlot>, E>;
}

impl<T> TryFindMethods<T> for hashbrown::raw::RawTable<T> {
    #[inline(always)]
    fn try_find<E>(
        &self,
        hash: u64,
        mut compare: impl FnMut(&T) -> Result<bool, E>,
    ) -> Result<Option<hashbrown::raw::Bucket<T>>, E> {
        let mut error = None;

        let found = self.find(hash, |item| {
            match compare(item) {
                Ok(boolean) => boolean,
                Err(e) => {
                    error = Some(e);
                    true // To break checking
                }
            }
        });

        if let Some(error) = error {
            Err(error)
        } else {
            Ok(found)
        }
    }

    #[inline(always)]
    fn try_find_or_find_insert_slot<E>(
        &mut self,
        hash: u64,
        mut compare: impl FnMut(&T) -> Result<bool, E>,
        hasher: impl Fn(&T) -> u64,
    ) -> Result<Result<hashbrown::raw::Bucket<T>, hashbrown::raw::InsertSlot>, E> {
        let mut error = None;

        let found = self.find_or_find_insert_slot(
            hash,
            |item| {
                match compare(item) {
                    Ok(boolean) => boolean,
                    Err(e) => {
                        error = Some(e);
                        true // To break checking
                    }
                }
            },
            hasher,
        );

        if let Some(error) = error {
            Err(error)
        } else {
            Ok(found)
        }
    }
}

/// Observe caches' changes
#[derive(Debug)]
pub struct Observed(std::num::NonZeroU16);

impl Observed {
    pub fn new() -> Self {
        Self(unsafe { std::num::NonZeroU16::new_unchecked(1) })
    }

    pub fn change(&mut self) {
        self.0 = self
            .0
            .checked_add(1)
            .or_else(|| Some(unsafe { std::num::NonZeroU16::new_unchecked(1) }))
            .unwrap();
    }

    pub fn get(&self) -> u16 {
        self.0.get()
    }
}

unsafe fn _get_state(py: pyo3::Python<'_>, ptr: *mut pyo3::ffi::PyObject) -> pyo3::PyResult<u16> {
    unsafe fn inner(
        py: pyo3::Python<'_>,
        ptr: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        cfg_if::cfg_if! {
            if #[cfg(all(Py_3_9, not(any(Py_LIMITED_API, PyPy, GraalPy))))] {
                use pyo3::IntoPyObject;

                let m_name: pyo3::Bound<'_, pyo3::types::PyString> = "_state".into_pyobject(py)?;
                Ok(pyo3::ffi::PyObject_CallMethodNoArgs(ptr, m_name.as_ptr()))
            } else {
                let state_fn =
                    pyo3::ffi::PyObject_GetAttrString(ptr, pyo3::ffi::c_str!("_state").as_ptr());

                if state_fn.is_null() {
                    return Err(pyo3::PyErr::take(py).unwrap_unchecked());
                }

                let empty_args = pyo3::ffi::PyTuple_New(0);
                let result = pyo3::ffi::PyObject_Call(state_fn, empty_args, std::ptr::null_mut());
                pyo3::ffi::Py_XDECREF(empty_args);
                pyo3::ffi::Py_XDECREF(state_fn);

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

    Ok(c as u16)
}

/// Checks the [`Observed`] on iterators
#[derive(Debug)]
pub struct ObservedIterator {
    pub ptr: core::ptr::NonNull<pyo3::ffi::PyObject>,
    pub statepoint: u16,
}

impl ObservedIterator {
    pub fn new(ptr: *mut pyo3::ffi::PyObject, state: u16) -> Self {
        unsafe {
            pyo3::ffi::Py_XINCREF(ptr);
        }

        Self {
            ptr: unsafe { core::ptr::NonNull::new(ptr).unwrap_unchecked() },
            statepoint: state,
        }
    }

    pub fn proceed(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<()> {
        let state = unsafe { _get_state(py, self.ptr.as_ptr())? };

        if state != self.statepoint {
            return Err(pyo3::PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "cache changed during iteration",
            ));
        }

        Ok(())
    }
}

impl Drop for ObservedIterator {
    fn drop(&mut self) {
        unsafe {
            pyo3::ffi::Py_XDECREF(self.ptr.as_ptr());
        }
    }
}

unsafe impl Send for ObservedIterator {}
unsafe impl Sync for ObservedIterator {}

pub struct NoLifetimeSliceIter<T> {
    pub pointer: std::ptr::NonNull<T>,
    pub index: usize,
    pub len: usize,
}

impl<T> NoLifetimeSliceIter<T> {
    #[inline]
    pub fn new(slice: &[T]) -> Self {
        let pointer: std::ptr::NonNull<T> = std::ptr::NonNull::from(slice).cast();

        Self {
            pointer,
            index: 0,
            len: slice.len(),
        }
    }
}

impl<T> Iterator for NoLifetimeSliceIter<T> {
    type Item = std::ptr::NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            None
        } else {
            let value = unsafe { self.pointer.add(self.index) };
            self.index += 1;
            Some(value)
        }
    }
}
