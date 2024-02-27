use pyo3::prelude::*;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::base::{self, CacheImplemention};

#[pyclass(extends=base::BaseCacheImpl, module="cachebox._cachebox", subclass)]
pub struct RRCache {
    inner: RwLock<HashMap<isize, base::KeyValuePair>>,
    maxsize: usize,
}

impl CacheImplemention for RRCache {
    type Pair = base::KeyValuePair;

    fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            RRCache {
                inner: RwLock::new(HashMap::with_capacity(cap)),
                maxsize,
            }
        } else {
            RRCache {
                inner: RwLock::new(HashMap::new()),
                maxsize,
            }
        }
    }

    fn cache_popitem(&mut self) -> Option<Self::Pair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        if write.is_empty() {
            None
        } else {
            let key = *write.keys().choose(&mut rand::thread_rng()).unwrap();
            write.remove(&key)
        }
    }

    fn cache_setitem(&mut self, hash: isize, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()> {
        if self.maxsize > 0 {
            let read = self.inner.read().expect("RwLock is poisoned (read)");
            let length = read.len() + 1;

            if length > self.maxsize && read.get(&hash).is_none() {
                drop(read);

                for _ in 0..(length - self.maxsize) {
                    if self.cache_popitem().is_none() {
                        break;
                    }
                }
            }
        }

        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        let length = write.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == write.capacity();

        write.insert(hash, base::KeyValuePair(key, value));

        if time_to_shrink {
            write.shrink_to_fit();
        }

        Ok(())
    }

    fn cache_remove(&mut self, hash: &isize) -> Option<Self::Pair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        write.remove(hash)
    }

    fn cache_len(&self) -> usize {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.len()
    }

    fn cache_contains(&self, hash: &isize) -> bool {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.contains_key(hash)
    }

    fn cache_clear(&mut self, reuse: bool) {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        write.clear();

        if !reuse {
            write.shrink_to_fit();
        }
    }

    fn cache_sizeof(&self) -> usize {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.capacity() * base::ISIZE_MEMORY_SIZE + base::ISIZE_MEMORY_SIZE
    }

    fn cache_keys(&self) -> Vec<Py<PyAny>> {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.values().map(|x| x.0.clone()).collect()
    }

    fn cache_values(&self) -> Vec<Py<PyAny>> {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.values().map(|x| x.1.clone()).collect()
    }

    fn cache_items(&self) -> Vec<(Py<PyAny>, Py<PyAny>)> {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.values().map(|x| (x.0.clone(), x.1.clone())).collect()
    }

    fn cache_equal(&self, other: &Self) -> bool {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        let other_read = other.inner.read().expect("RwLock is poisoned (read)");

        read.len() == other_read.len() && read.keys().all(|x| other_read.contains_key(x))
    }

    fn cache_update_from_pydict(&mut self, other: &pyo3::types::PyDict) -> PyResult<()> {
        for i in other.items() {
            let items: (&PyAny, &PyAny) = i.extract()?;
            self.cache_setitem(items.0.hash()?, items.0.into(), items.1.into())?;
        }

        Ok(())
    }

    fn cache_update_from_pyobject(&mut self, other: &pyo3::types::PyIterator) -> PyResult<()> {
        for i in other {
            let items: (&PyAny, &PyAny) = i?.extract()?;
            self.cache_setitem(items.0.hash()?, items.0.into(), items.1.into())?;
        }

        Ok(())
    }
}


#[pymethods]
impl RRCache {
    pub fn __getitem__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        let read = self.inner.read().expect("RwLock is poisoned (read)");

        match read.get(&hash) {
            Some(v) => Ok(v.1.clone()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(format!("{}", key))),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn get(
        &self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        let read = self.inner.read().expect("RwLock is poisoned (read)");

        match read.get(&hash) {
            Some(v) => Ok(Some(v.1.clone())),
            None => Ok(default),
        }
    }

    pub fn popitem(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
        match self.cache_popitem() {
            None => Err(pyo3::exceptions::PyKeyError::new_err("cache is empty")),
            Some(v) => Ok((v.0, v.1)),
        }
    }
}

crate::implement_default_functions!(RRCache);
