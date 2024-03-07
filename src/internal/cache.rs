use std::collections::HashMap;

pub struct Cache<K: Sized, V> {
    inner: HashMap<K, V>,
    pub maxsize: usize,
}

impl<K, V> Cache<K, V> {
    #[inline]
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

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<K: std::hash::Hash + Eq, V> Cache<K, V> {
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            return Err(pyo3::exceptions::PyOverflowError::new_err(
                "The cache reached maximum size",
            ));
        }

        self.inner.insert(key, value);

        Ok(())
    }

    #[inline]
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    #[inline]
    pub fn clear(&mut self, reuse: bool) {
        self.inner.clear();

        if !reuse {
            self.inner.shrink_to_fit();
        }
    }

    #[inline]
    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.inner.keys()
    }

    #[inline]
    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
        self.inner.values()
    }

    #[inline]
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.inner.iter()
    }

    #[inline]
    pub fn drain(&mut self) -> std::collections::hash_map::Drain<'_, K, V> {
        self.inner.drain()
    }

    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }
}

impl<K: std::hash::Hash + Eq, V: Clone> Cache<K, V> {
    #[inline]
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

impl<K: std::hash::Hash + Eq, V> Cache<K, V> {
    #[inline]
    pub fn update<T: IntoIterator<Item = pyo3::PyResult<(K, V)>>>(
        &mut self,
        iterable: T,
    ) -> pyo3::PyResult<()> {
        for result in iterable {
            let (key, val) = result?;
            if self.maxsize > 0
                && self.inner.len() >= self.maxsize
                && self.inner.get(&key).is_none()
            {
                return Err(pyo3::exceptions::PyOverflowError::new_err(
                    "The cache reached maximum size",
                ));
            }

            self.inner.insert(key, val);
        }

        Ok(())
    }
}

impl<K: Clone, V: Clone> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Cache {
            inner: self.inner.clone(),
            maxsize: self.maxsize,
        }
    }
}
