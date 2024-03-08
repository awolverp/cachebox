use std::collections::{HashMap, BinaryHeap};

pub struct LFUCache<K, V> {
    inner: std::collections::HashMap<K, V>,
    counter: std::collections::HashMap<K, usize>,
    pub maxsize: usize,
}

impl<K, V> LFUCache<K, V> {
    #[must_use]
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if maxsize > 0 && capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            return LFUCache {
                inner: HashMap::with_capacity(cap),
                counter: HashMap::with_capacity(cap),
                maxsize,
            };
        }

        LFUCache {
            inner: HashMap::new(),
            counter: HashMap::new(),
            maxsize,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn counter_capacity(&self) -> usize {
        self.counter.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}


impl<K: std::hash::Hash + Eq + std::cmp::Ord + Copy, V> LFUCache<K, V> {
    pub fn popitem(&mut self) -> Option<V> {
        if self.inner.is_empty() {
            None
        } else {
            let heap: BinaryHeap<_> = self.counter.iter().map(|(t, n)| (std::cmp::Reverse(*n), *t)).collect();
            let (std::cmp::Reverse(_), least_frequently_used_key) = heap.peek().unwrap();

            self.counter.remove(least_frequently_used_key);
            self.inner.remove(least_frequently_used_key)
        }
    }

    pub fn least_frequently_used(&self) -> Option<K> {
        if self.inner.is_empty() {
            None
        } else {
            let heap: BinaryHeap<_> = self.counter.iter().map(|(t, n)| (std::cmp::Reverse(*n), *t)).collect();
            let (std::cmp::Reverse(_), least_frequently_used_key) = heap.peek().unwrap();

            Some(*least_frequently_used_key)
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self.inner.insert(key.clone(), value) {
            Some(_) => {
                *self.counter.get_mut(&key).unwrap() += 1;
            },
            None => {
                self.counter.insert(key, 0);
            }
        }

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(())
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

impl<K: std::hash::Hash + Eq, V> LFUCache<K, V> {
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
        self.counter.shrink_to_fit();
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        match self.inner.remove(key) {
            Some(val) => {
                self.counter.remove(key);
                Some(val)
            }
            None => None,
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    pub fn clear(&mut self, reuse: bool) {
        self.inner.clear();
        self.counter.clear();

        if !reuse {
            self.inner.shrink_to_fit();
            self.counter.shrink_to_fit();
        }
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.inner.keys()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
        self.inner.values()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.inner.iter()
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        match self.inner.get(key) {
            Some(val) => {
                *self.counter.get_mut(key).unwrap() += 1;
                Some(val)
            }
            None => {
                None
            }
        }
    }
}

impl<K: std::hash::Hash + Eq + std::cmp::Ord + Copy, V: Clone> LFUCache<K, V> {
    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.inner.get(&key);
        if exists.is_some() {
            return Ok(exists.cloned().unwrap());
        }

        if self.maxsize > 0 && self.inner.len() >= self.maxsize {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        self.inner.insert(key.clone(), default.clone());
        self.counter.insert(key, 0);

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(default)
    }
}

impl<K: PartialEq + std::cmp::Eq + std::hash::Hash, V> PartialEq for LFUCache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.counter == other.counter
    }
}
impl<K: PartialEq + std::cmp::Eq + std::hash::Hash, V> Eq for LFUCache<K, V> {}

