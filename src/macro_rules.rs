/// Implements a `#[pyclass]` with pre-defined pyclass arguments.
///
/// # Example
///
/// ```ignore
/// implement_pyclass! {
///     [] MyClass as "MyClass" { field: type }
/// }
/// ```
#[macro_export]
macro_rules! implement_pyclass {
    (
        $(#[$outer:meta])*
        [$($pyclass_args:tt)*] $struct_name:ident as $python_name:literal $($rest:tt)*
    ) => {
        #[pyo3::pyclass(
            module = "cachebox._core",
            name = $python_name,
            immutable_type,
            $($pyclass_args)*
        )]
        $(#[$outer])*
        pub struct $struct_name $($rest)*
    };
}

/// Implements the guard shared by every cache view type.
///
/// Views hold raw positions into the policy storage. A position is valid only while
/// the cache is unchanged, and every mutation bumps the generation version while
/// holding the cache lock. So a view must take the cache lock **first**, then compare
/// the generation, and keep the lock while it reads through its position. Checking
/// without the lock races with other threads on free-threaded builds (#94).
///
/// Lock order: cache lock, then the view's own iterator mutex.
///
/// # Example
///
/// ```ignore
/// implement_view_guard!(PyCacheKeys, nopolicy::NoPolicy);
/// ```
#[macro_export]
macro_rules! implement_view_guard {
    ($name:ident, $policy:ty) => {
        impl $name {
            /// Locks the cache and fails if it changed after this view was created.
            ///
            /// Hold the returned guard for as long as entries are read.
            #[inline]
            fn lock_and_check<'a>(
                &'a self,
                py: pyo3::Python<'_>,
            ) -> pyo3::PyResult<parking_lot::MutexGuard<'a, $policy>> {
                let cache = self.cache.get();
                let inner = cache.0.get()?;

                let policy = inner.policy(py);

                if self.initial_gv == inner.shared().generation_version().get() {
                    return Ok(policy);
                }

                Err($crate::new_py_error!(
                    PyRuntimeError,
                    "cache size changed during iteration"
                ))
            }
        }
    };
}

/// Creates a new [`PyErr`] of the given exception type.
#[macro_export]
macro_rules! new_py_error {
    ($name:ident, $msg:expr $(,)?) => {
        ::pyo3::exceptions::$name::new_err($msg)
    };
    ($name:ident, $fmt:expr, $($args:tt)*) => {
        ::pyo3::exceptions::$name::new_err(
            format_args!($fmt, $($args)*)
        )
    };
}

/// Creates a new std::num::NonZeroUsize safely. Uses `isize::MAX as usize` when `num` is zero.
///
/// # Usage
///
/// ```ignore
/// safe_non_zero!(2) -> std::num::NonZeroUsize(2)
/// safe_non_zero!(0) -> std::num::NonZeroUsize(isize::MAX as usize)
/// ```
#[macro_export]
macro_rules! safe_non_zero {
    ($num:expr) => {
        std::num::NonZeroUsize::new(if $num == 0 { isize::MAX as usize } else { $num }).unwrap()
    };
}
