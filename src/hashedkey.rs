#[derive(Debug)]
pub struct HashedKey {
    pub key: pyo3::PyObject,

    // The `key` hash in Rust.
    // Why u64? because hash type in Rust is u64 and hashbrown only accepts u64 as hash,
    // I didn't found any better way.
    pub hash: u64,
}

impl HashedKey {
    #[inline]
    pub fn from_key_and_hash(key: pyo3::PyObject, hash: u64) -> Self {
        Self { key, hash }
    }

    #[inline]
    pub fn from_pyobject(py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<Self> {
        unsafe {
            let py_hash = pyo3::ffi::PyObject_Hash(key.as_ptr());

            if py_hash == -1 {
                // There's no need to check PyErr_Occurred,
                // PyObject_Hash never returns -1 when success.
                return Err(pyo3::PyErr::take(py).unwrap());
            }

            Ok(Self::from_key_and_hash(key, fxhash::hash64(&py_hash)))
        }
    }

    pub fn clone_ref(&self, py: pyo3::Python<'_>) -> Self {
        Self {
            key: self.key.clone_ref(py),
            hash: self.hash,
        }
    }
}

impl PartialEq for HashedKey {
    fn eq(&self, other: &Self) -> bool {
        pyobject_eq!(self.key, other.key)
    }
}

impl Eq for HashedKey {}
