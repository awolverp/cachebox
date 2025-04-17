use pyo3::create_exception;

create_exception!(cachebox._core, CoreKeyError, pyo3::exceptions::PyException);

#[pyo3::pyclass(module = "cachebox._core", frozen)]
pub struct TTLPair {
    key: pyo3::PyObject,
    value: pyo3::PyObject,
    duration: std::time::Duration,
}

impl TTLPair {
    fn clone_from_pair(py: pyo3::Python<'_>, pair: &crate::common::TimeToLivePair) -> Self {
        TTLPair {
            key: pair.key.obj.clone_ref(py),
            value: pair.value.clone_ref(py),
            duration: pair.duration().unwrap_or_default(),
        }
    }
}

impl From<crate::common::TimeToLivePair> for TTLPair {
    fn from(value: crate::common::TimeToLivePair) -> Self {
        let duration = value.duration().unwrap_or_default();

        TTLPair {
            key: value.key.obj,
            value: value.value,
            duration,
        }
    }
}

#[pyo3::pymethods]
impl TTLPair {
    fn key(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyObject {
        slf.key.clone_ref(slf.py())
    }

    fn value(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyObject {
        slf.value.clone_ref(slf.py())
    }

    fn duration(slf: pyo3::PyRef<'_, Self>) -> f64 {
        slf.duration.as_secs_f64()
    }

    fn pack2(slf: pyo3::PyRef<'_, Self>) -> (pyo3::PyObject, pyo3::PyObject) {
        (slf.key.clone_ref(slf.py()), slf.value.clone_ref(slf.py()))
    }

    fn pack3(slf: pyo3::PyRef<'_, Self>) -> (pyo3::PyObject, pyo3::PyObject, f64) {
        (
            slf.key.clone_ref(slf.py()),
            slf.value.clone_ref(slf.py()),
            slf.duration.as_secs_f64(),
        )
    }
}

pub mod cache;
pub mod fifocache;
pub mod lfucache;
pub mod lrucache;
pub mod rrcache;
pub mod ttlcache;
pub mod vttlcache;
