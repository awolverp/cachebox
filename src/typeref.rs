pub static mut STD_DICT_TYPE: *mut pyo3::ffi::PyTypeObject = std::ptr::null_mut();
pub static mut STD_TUPLE_TYPE: *mut pyo3::ffi::PyTypeObject = std::ptr::null_mut();

unsafe fn get_type_object_for<T: pyo3::PyTypeInfo>(
    py: pyo3::Python,
) -> *mut pyo3::ffi::PyTypeObject {
    T::type_object_raw(py)
}

#[cold]
#[optimize(size)]
fn _initialize_typeref(py: pyo3::Python) {
    unsafe {
        STD_DICT_TYPE = get_type_object_for::<pyo3::types::PyDict>(py);
        STD_TUPLE_TYPE = get_type_object_for::<pyo3::types::PyTuple>(py);
    }
}

pub fn initialize_typeref(py: pyo3::Python) {
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| _initialize_typeref(py));
}
