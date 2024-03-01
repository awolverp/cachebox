use pyo3::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;

use crate::base::{self, CacheImplemention};

#[pyclass(extends=base::BaseCacheImpl, module="cachebox._cachebox", subclass)]
pub struct MRUCache {
    inner: RwLock<HashMap<isize, base::KeyValuePair>>,
    order: RwLock<VecDeque<isize>>,
    maxsize: usize,
}

macro_rules! lru_move_to_front {
    ($order:expr, $key:expr) => {{
        let index = $order.iter().rev().position(|x| *x == $key).unwrap();
        let index = $order.len() - index - 1;
        let item = $order.remove(index).unwrap();
        $order.push_front(item);
    }};
}

impl CacheImplemention for MRUCache {
    type Pair = base::KeyValuePair;

    fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            MRUCache {
                inner: RwLock::new(HashMap::with_capacity(cap)),
                order: RwLock::new(VecDeque::with_capacity(cap)),
                maxsize,
            }
        } else {
            MRUCache {
                inner: RwLock::new(HashMap::new()),
                order: RwLock::new(VecDeque::new()),
                maxsize,
            }
        }
    }

    fn cache_popitem(&mut self) -> Option<Self::Pair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        let mut order = self
            .order
            .write()
            .expect("RwLock is poisoned (write/order)");

        match order.pop_front() {
            Some(key) => write.remove(&key),
            None => None,
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
        let mut order = self
            .order
            .write()
            .expect("RwLock is poisoned (write/order)");
        let length = write.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == write.capacity();

        match write.insert(hash, base::KeyValuePair(key, value)) {
            Some(_) => lru_move_to_front!(order, hash),
            None => order.push_front(hash),
        }

        if time_to_shrink {
            write.shrink_to_fit();
        }

        Ok(())
    }

    fn cache_remove(&mut self, hash: &isize) -> Option<Self::Pair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        let mut order = self
            .order
            .write()
            .expect("RwLock is poisoned (write/order)");

        match write.remove(hash) {
            Some(v) => {
                let index = order.iter().position(|x| *x == *hash).unwrap();
                order.remove(index);
                Some(v)
            }
            None => None,
        }
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
        let mut order = self
            .order
            .write()
            .expect("RwLock is poisoned (write/order)");
        write.clear();
        order.clear();

        if !reuse {
            write.shrink_to_fit();
            order.shrink_to_fit();
        }
    }

    fn cache_sizeof(&self) -> usize {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        let order = self.order.read().expect("RwLock is poisoned (read/order)");

        read.capacity() * base::ISIZE_MEMORY_SIZE
            + order.capacity() * base::ISIZE_MEMORY_SIZE
            + base::ISIZE_MEMORY_SIZE
    }

    fn cache_keys(&self) -> Vec<Py<PyAny>> {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        let order = self.order.read().expect("RwLock is poisoned (read/order)");

        order
            .iter()
            .map(|x| read.get(x).unwrap().0.clone())
            .collect()
    }

    fn cache_values(&self) -> Vec<Py<PyAny>> {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        let order = self.order.read().expect("RwLock is poisoned (read/order)");

        order
            .iter()
            .map(|x| read.get(x).unwrap().1.clone())
            .collect()
    }

    fn cache_items(&self) -> Vec<(Py<PyAny>, Py<PyAny>)> {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        let order = self.order.read().expect("RwLock is poisoned (read/order)");

        order
            .iter()
            .map(|x| {
                let y = read.get(x).unwrap();
                (y.0.clone(), y.1.clone())
            })
            .collect()
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
impl MRUCache {
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
            Some(v) => {
                let mut order = self
                    .order
                    .write()
                    .expect("RwLock is poisoned (write/order)");

                lru_move_to_front!(order, hash);
                Ok(v.1.clone())
            }
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
            Some(v) => {
                let mut order = self
                    .order
                    .write()
                    .expect("RwLock is poisoned (write/order)");

                lru_move_to_front!(order, hash);
                Ok(Some(v.1.clone()))
            }
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

crate::implement_default_functions!(MRUCache);
