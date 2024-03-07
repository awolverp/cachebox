use std::collections::HashMap;

pub struct Cache<K: Sized, V> {
    inner: HashMap<K, V>,
    pub maxsize: usize,
}

impl<K, V> Cache<K, V> {
    #[must_use]
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if maxsize > 0 && capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            return Cache {
                inner: HashMap::with_capacity(cap),
                maxsize,
            };
        }

        Cache {
            inner: HashMap::new(),
            maxsize,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<K: std::hash::Hash + Eq, V> Cache<K, V> {
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            return Err(pyo3::exceptions::PyOverflowError::new_err(
                "The cache reached maximum size",
            ));
        }

        self.inner.insert(key, value);

        Ok(())
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    pub fn clear(&mut self, reuse: bool) {
        self.inner.clear();

        if !reuse {
            self.inner.shrink_to_fit();
        }
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.inner.keys()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
        self.inner.values()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    pub fn update<T: IntoIterator<Item = pyo3::PyResult<(K, V)>>>(
        &mut self,
        iterable: T,
    ) -> pyo3::PyResult<()> {
        for result in iterable {
            let (key, val) = result?;
            self.insert(key, val)?;
        }

        Ok(())
    }
}

impl<K: std::hash::Hash + Eq, V: Clone> Cache<K, V> {
    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.inner.get(&key);
        if exists.is_some() {
            return Ok(exists.cloned().unwrap());
        }

        if self.maxsize > 0 && self.inner.len() >= self.maxsize {
            return Err(pyo3::exceptions::PyOverflowError::new_err(
                "The cache reached maximum size",
            ));
        }

        self.inner.insert(key, default.clone());
        Ok(default)
    }
}
