use pyo3::prelude::*;
use std::sync::RwLock;

use crate::classes::base;
use crate::internal;

#[pyclass(extends=base::BaseCacheImpl, subclass, module = "cachebox._cachebox")]
pub struct Cache {
    inner: RwLock<internal::Cache<isize, base::KeyValuePair>>,
}

#[pymethods]
impl Cache {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0))]
    fn __new__(maxsize: usize, capacity: usize) -> (Self, base::BaseCacheImpl) {
        (
            Cache {
                inner: RwLock::new(internal::Cache::new(maxsize, capacity)),
            },
            base::BaseCacheImpl {},
        )
    }

    #[getter]
    fn maxsize(&self) -> usize {
        let read = use_rwlock!(r self.inner);
        read.maxsize
    }

    fn __len__(&self) -> usize {
        let read = use_rwlock!(r self.inner);
        read.len()
    }

    fn __sizeof__(&self) -> usize {
        let read = use_rwlock!(r self.inner);
        read.capacity() * base::ISIZE_MEMORY_SIZE + base::ISIZE_MEMORY_SIZE
    }

    fn __bool__(&self) -> bool {
        let read = use_rwlock!(r self.inner);
        !read.is_empty()
    }

    fn __setitem__(&mut self, py: Python<'_>, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()> {
        let hash = pyany_to_hash!(key, py);
        if let Err(e) = hash {
            return Err(e);
        }
        let hash = hash.unwrap();

        let mut write = use_rwlock!(w self.inner);
        if let Err(s) = write.insert(hash, base::KeyValuePair(key, value)) {
            return Err(pyo3::exceptions::PyOverflowError::new_err(s));
        }

        Ok(())
    }

    fn __getitem__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let hash = pyany_to_hash!(key, py);
        if let Err(e) = hash {
            return Err(e);
        }
        let hash = hash.unwrap();

        let read = use_rwlock!(r self.inner);
        match read.get(&hash) {
            Some(x) => Ok(x.1.clone()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key)),
        }
    }

    fn __delitem__(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        let hash = pyany_to_hash!(key, py);
        if let Err(e) = hash {
            return Err(e);
        }
        let hash = hash.unwrap();

        let mut write = use_rwlock!(w self.inner);
        match write.remove(&hash) {
            Some(_) => Ok(()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key)),
        }
    }

    fn __contains__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<bool> {
        let hash = pyany_to_hash!(key, py);
        if let Err(e) = hash {
            return Err(e);
        }
        let hash = hash.unwrap();

        let read = use_rwlock!(r self.inner);
        Ok(read.contains_key(&hash))
    }

    fn __eq__(&self, other: &Self) -> bool {
        let read1 = use_rwlock!(r self.inner);
        let read2 = use_rwlock!(r other.inner);
        read1.maxsize == read2.maxsize && read1.keys().all(|x| read2.contains_key(x))
    }

    fn __ne__(&self, other: &Self) -> bool {
        let read1 = use_rwlock!(r self.inner);
        let read2 = use_rwlock!(r other.inner);
        read1.maxsize != read2.maxsize || read1.keys().all(|x| !read2.contains_key(x))
    }

    fn __repr__(&self) -> String {
        let read = use_rwlock!(r self.inner);
        format!(
            "<cachebox._cachebox.Cache len={} maxsize={} capacity={}>",
            read.len(),
            read.maxsize,
            read.capacity()
        )
    }

    fn capacity(&self) -> usize {
        let read = use_rwlock!(r self.inner);
        read.capacity()
    }

    #[pyo3(signature=(*, reuse=false))]
    fn clear(&self, reuse: bool) {
        let mut write = use_rwlock!(w self.inner);
        write.clear(reuse);
    }

    #[pyo3(signature=(*, key, default=None))]
    fn pop(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let hash = pyany_to_hash!(key, py);
        if let Err(e) = hash {
            return Err(e);
        }
        let hash = hash.unwrap();

        let mut write = use_rwlock!(w self.inner);
        match write.remove(&hash) {
            Some(x) => Ok(Some(x.1)),
            None => Ok(default),
        }
    }
}
