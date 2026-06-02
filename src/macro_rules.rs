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
            skip_from_py_object,
            $($pyclass_args)*
        )]
        $(#[$outer])*
        pub struct $struct_name $($rest)*
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
