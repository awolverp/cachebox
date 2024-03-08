use rand::seq::IteratorRandom;

use super::cache::Cache;

pub struct RRCache<K, V> {
    pub parent: Cache<K, V>,
}

impl<K, V> RRCache<K, V> {
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        Self { parent: Cache::new(maxsize, capacity) }
    }
}

impl<K: std::hash::Hash + Eq + Copy, V> RRCache<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.parent.maxsize > 0 && self.parent.inner.len() >= self.parent.maxsize && self.parent.inner.get(&key).is_none() {
            self.popitem();
        }

        self.parent.inner.insert(key, value);

        Ok(())
    }

    pub fn popitem(&mut self) -> Option<V> {
        if self.parent.is_empty() {
            None
        } else {
            let key = *self.parent.keys().choose(&mut rand::thread_rng()).unwrap();
            self.parent.remove(&key)
        }
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

impl<K: std::hash::Hash + Eq + Copy, V: Clone> RRCache<K, V> {
    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.parent.inner.get(&key);
        if exists.is_some() {
            return Ok(exists.cloned().unwrap());
        }

        if self.parent.maxsize > 0 && self.parent.inner.len() >= self.parent.maxsize {
            self.popitem();
        }

        self.parent.inner.insert(key, default.clone());
        Ok(default)
    }
}
