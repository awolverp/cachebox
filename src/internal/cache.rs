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
            return Cache {
                inner: HashMap::with_capacity(
                    if capacity <= maxsize { capacity } else { maxsize }
                ),
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
    #[must_use]
    pub fn insert(&mut self, key: K, value: V) -> Result<(), String> {
        if self.maxsize > 0 {
            if self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
                return Err(String::from("The cache reached maximum size"));
            }
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
    pub fn reserve(&mut self, additional: usize) -> Result<(), std::collections::TryReserveError> {
        self.inner.try_reserve(additional)
    }

    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }

    pub fn update<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) -> Result<(), String> {
        for (key, val) in iter {
            self.insert(key, val)?;
        }

        Ok(())
    }
}

impl<K: std::hash::Hash + Eq + Copy, V> Cache<K, V> {
    #[inline]
    pub fn popitem(&mut self) -> Option<(K, V)> {
        if let Some(key) = self.inner.keys().next().cloned() {
            let value = self.inner.remove(&key).unwrap();
            return Some((key, value));
        }

        None
    }
}

impl<K: Eq + std::hash::Hash, V: PartialEq> PartialEq for Cache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.inner == other.inner
    }
}
impl<K: Eq + std::hash::Hash, V: PartialEq> Eq for Cache<K, V> {}
