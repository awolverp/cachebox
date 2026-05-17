/// It can use as PyO3 function argument. When an argument is specified, you will get [`OptionalArgument::Defined`],
/// otherwise you will get [`OptionalArgument::Undefined`].
///
/// It can be used instead of [`Option<T>`] to improve performance.
pub enum OptionalArgument<'a> {
    /// The argument was not provided by the caller.
    Undefined,
    /// The argument was provided and holds the bound Python object.
    Defined(pyo3::Bound<'a, pyo3::PyAny>),
}

impl<'a, 'py> pyo3::FromPyObject<'a, 'py> for OptionalArgument<'py> {
    type Error = pyo3::PyErr;

    fn extract(obj: pyo3::Borrowed<'a, 'py, pyo3::PyAny>) -> Result<Self, Self::Error> {
        Ok(Self::Defined(obj.to_owned()))
    }
}
