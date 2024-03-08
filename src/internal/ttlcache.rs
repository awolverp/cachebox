use std::collections::{HashMap, VecDeque};
use std::time;

#[derive(Clone)]
pub struct Value<T: Clone> {
    pub value: T,
    pub expiration: time::Instant,
}

impl<T: Clone> Value<T> {
    #[must_use]
    fn new(value: T, ttl: time::Duration) -> Self {
        Value {
            value,
            expiration: time::Instant::now() + ttl,
        }
    }

    pub fn expired(&self) -> bool {
        time::Instant::now() > self.expiration
    }
}

pub struct TTLCache<K, V: Clone> {
    inner: HashMap<K, Value<V>>,
    order: VecDeque<K>,
    pub ttl: time::Duration,
    pub maxsize: usize,
}

impl<K, V: Clone> TTLCache<K, V> {
    #[must_use]
    pub fn new(maxsize: usize, ttl: f32, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if maxsize > 0 && capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            return Self {
                inner: HashMap::with_capacity(cap),
                order: VecDeque::with_capacity(cap),
                ttl: time::Duration::from_secs_f32(ttl),
                maxsize,
            };
        }

        Self {
            inner: HashMap::new(),
            order: VecDeque::new(),
            ttl: time::Duration::from_secs_f32(ttl),
            maxsize,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn order_capacity(&self) -> usize {
        self.order.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> TTLCache<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self.inner.insert(key.clone(), Value::new(value, self.ttl)) {
            Some(_) => (),
            None => self.order.push_back(key),
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

    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.inner.get(&key);
        if exists.is_some() {
            let val = unsafe { exists.unwrap_unchecked() };
            if !val.expired() {
                return Ok(val.clone().value);
            }
        }

        if self.maxsize > 0 && self.inner.len() >= self.maxsize {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        self.inner
            .insert(key.clone(), Value::new(default.clone(), self.ttl));
        self.order.push_back(key);

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(default)
    }
}

impl<K: std::hash::Hash + Eq, V: Clone> TTLCache<K, V> {
    pub fn expire(&mut self) {
        while let Some(key) = self.order.front() {
            if !self.inner[key].expired() {
                break;
            }

            self.inner.remove(key);
            self.order.pop_front();
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    pub fn popitem(&mut self) -> Option<Value<V>> {
        match self.order.pop_front() {
            Some(x) => self.inner.remove(&x),
            None => None,
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<Value<V>> {
        match self.inner.remove(key) {
            Some(val) => {
                let index = unsafe { self.order.iter().position(|x| x == key).unwrap_unchecked() };
                self.order.remove(index);

                if val.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        match self.inner.get(key) {
            Some(val) => !val.expired(),
            None => false,
        }
    }

    pub fn clear(&mut self, reuse: bool) {
        self.inner.clear();
        self.order.clear();

        if !reuse {
            self.inner.shrink_to_fit();
            self.order.shrink_to_fit();
        }
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, Value<V>> {
        self.inner.keys()
    }

    pub fn sorted_keys(&self) -> std::collections::vec_deque::Iter<'_, K> {
        self.order.iter()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, Value<V>> {
        self.inner.values()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, Value<V>> {
        self.inner.iter()
    }

    pub fn get(&self, key: &K) -> Option<&Value<V>> {
        self.inner.get(key).filter(|&val| !val.expired())
    }
}

impl<K: PartialEq + std::hash::Hash + Eq, V: Clone> PartialEq for TTLCache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.order == other.order
    }
}
impl<K: PartialEq + std::hash::Hash + Eq, V: Clone> Eq for TTLCache<K, V> {}
