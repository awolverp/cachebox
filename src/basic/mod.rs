#[macro_use]
mod hashing;
mod baseclass;
pub mod iter;

pub use baseclass::BaseCacheImpl;
pub use hashing::HashablePyObject;

pub const PYOBJECT_MEM_SIZE: usize = core::mem::size_of::<pyo3::PyObject>();

#[macro_export]
macro_rules! create_pyerr {
    ($err:ty, $val:expr) => {
        pyo3::PyErr::new::<$err, _>($val)
    };

    ($err:ty) => {
        pyo3::PyErr::new::<$err, _>(())
    };
}
