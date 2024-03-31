use rand::seq::IteratorRandom;
use std::collections::VecDeque;
use std::time;

use ahash::AHashMap;

/// Fixed-size (or can be not) cache implementation without any policy,
/// So only can be fixed-size, or unlimited size cache
pub struct Cache<K, V> {
    pub(in crate::internal) inner: AHashMap<K, V>,
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

            return Self {
                inner: AHashMap::with_capacity(cap),
                maxsize,
            };
        }

        Self {
            inner: AHashMap::new(),
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

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.inner.iter()
    }

    #[inline(always)]
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
            return Ok(unsafe { exists.cloned().unwrap_unchecked() });
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

/// FIFO Cache implementation (First-In First-Out policy, very useful cache policy).
///
/// In simple terms, the FIFO cache will remove the element that has been in the cache the longest;
/// It behaves like a Python dictionary.
pub struct FIFOCache<K, V> {
    inner: AHashMap<K, V>,
    order: VecDeque<K>,
    pub maxsize: usize,
}

impl<K, V> FIFOCache<K, V> {
    #[must_use]
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if maxsize > 0 && capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            return Self {
                inner: AHashMap::with_capacity(cap),
                order: VecDeque::with_capacity(cap),
                maxsize,
            };
        }

        Self {
            inner: AHashMap::new(),
            order: VecDeque::new(),
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

impl<K: std::hash::Hash + Eq + Clone, V> FIFOCache<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self.inner.insert(key.clone(), value) {
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
}

impl<K: std::hash::Hash + Eq, V> FIFOCache<K, V> {
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    pub fn popitem(&mut self) -> Option<V> {
        match self.order.pop_front() {
            Some(x) => self.inner.remove(&x),
            None => None,
        }
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut c = 0usize;
        for _ in 0..n {
            if self.popitem().is_none() {
                break;
            }
            c += 1;
        }
        c
    }

    pub fn first(&self) -> Option<&K> {
        self.order.front()
    }

    pub fn last(&self) -> Option<&K> {
        self.order.back()
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        match self.inner.remove(key) {
            Some(val) => {
                let index = unsafe { self.order.iter().position(|x| x == key).unwrap_unchecked() };
                self.order.remove(index);
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
        self.order.clear();

        if !reuse {
            self.inner.shrink_to_fit();
            self.order.shrink_to_fit();
        }
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.inner.keys()
    }

    pub fn sorted_keys(&self) -> std::collections::vec_deque::Iter<'_, K> {
        self.order.iter()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
        self.inner.values()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.inner.iter()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> FIFOCache<K, V> {
    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.inner.get(&key);
        if exists.is_some() {
            return Ok(unsafe { exists.cloned().unwrap_unchecked() });
        }

        if self.maxsize > 0 && self.inner.len() >= self.maxsize {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        self.inner.insert(key.clone(), default.clone());
        self.order.push_back(key);

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(default)
    }
}

impl<K: PartialEq, V> PartialEq for FIFOCache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.order == other.order
    }
}
impl<K: PartialEq, V> Eq for FIFOCache<K, V> {}

/// LFU Cache implementation (Least frequantly used policy).
///
/// In simple terms, the LFU cache will remove the element in the cache that has been accessed the least, regardless of time.
pub struct LFUCache<K, V> {
    inner: AHashMap<K, V>,
    counter: AHashMap<K, usize>,
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

            return Self {
                inner: AHashMap::with_capacity(cap),
                counter: AHashMap::with_capacity(cap),
                maxsize,
            };
        }

        Self {
            inner: AHashMap::new(),
            counter: AHashMap::new(),
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
            let mut vector: Vec<_> = self.counter.iter().map(|(t, n)| (*n, *t)).collect();
            // vector.sort_unstable_by_key(|(n, _)| *n);
            vector.sort_unstable_by(|(n, _), (m, _)| n.cmp(m));

            let (_, least_frequently_used_key) = vector[0];

            self.counter.remove(&least_frequently_used_key);
            self.inner.remove(&least_frequently_used_key)
        }
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut c = 0usize;
        for _ in 0..n {
            if self.popitem().is_none() {
                break;
            }
            c += 1;
        }
        c
    }

    pub fn least_frequently_used(&self) -> Option<K> {
        if self.inner.is_empty() {
            None
        } else {
            let mut vector: Vec<_> = self.counter.iter().map(|(t, n)| (*n, *t)).collect();
            // vector.sort_unstable_by_key(|(n, _)| *n);
            vector.sort_unstable_by(|(n, _), (m, _)| n.cmp(m));
            
            let (_, least_frequently_used_key) = vector[0];
            Some(least_frequently_used_key)
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self.inner.insert(key, value) {
            Some(_) => {
                *self.counter.get_mut(&key).unwrap() += 1;
            }
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
            None => None,
        }
    }
}

impl<K: std::hash::Hash + Eq + std::cmp::Ord + Copy, V: Clone> LFUCache<K, V> {
    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.inner.get(&key);
        if exists.is_some() {
            return Ok(unsafe { exists.cloned().unwrap_unchecked() });
        }

        if self.maxsize > 0 && self.inner.len() >= self.maxsize {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        self.inner.insert(key, default.clone());
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

/// RRCache implementation (Random Replacement policy)
///
/// In simple terms, the RR cache will choice randomly element to remove it to make space when necessary.
pub struct RRCache<K, V> {
    pub parent: Cache<K, V>,
}

impl<K, V> RRCache<K, V> {
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        Self {
            parent: Cache::new(maxsize, capacity),
        }
    }
}

impl<K: std::hash::Hash + Eq + Copy, V> RRCache<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.parent.maxsize > 0
            && self.parent.inner.len() >= self.parent.maxsize
            && self.parent.inner.get(&key).is_none()
        {
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

    pub fn drain(&mut self, n: usize) -> usize {
        let mut c = 0usize;
        for _ in 0..n {
            if self.popitem().is_none() {
                break;
            }
            c += 1;
        }
        c
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
            return Ok(unsafe { exists.cloned().unwrap_unchecked() });
        }

        if self.parent.maxsize > 0 && self.parent.inner.len() >= self.parent.maxsize {
            self.popitem();
        }

        self.parent.inner.insert(key, default.clone());
        Ok(default)
    }
}

/// LRU Cache implementation (Least recently used policy)
///
/// In simple terms, the LRU cache will remove the element in the cache that has not been accessed in the longest time.
pub struct LRUCache<K, V> {
    inner: AHashMap<K, V>,
    order: VecDeque<K>,
    pub maxsize: usize,
}

macro_rules! vecdeque_move_to_end {
    ($order:expr, $key:expr) => {{
        let index = $order.iter().position(|x| *x == $key).unwrap();
        let item = unsafe { $order.remove(index).unwrap_unchecked() };
        $order.push_back(item);
    }};
}

impl<K, V> LRUCache<K, V> {
    #[must_use]
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if maxsize > 0 && capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            return Self {
                inner: AHashMap::with_capacity(cap),
                order: VecDeque::with_capacity(cap),
                maxsize,
            };
        }

        Self {
            inner: AHashMap::new(),
            order: VecDeque::new(),
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

impl<K: std::hash::Hash + Eq + Clone, V> LRUCache<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self.inner.insert(key.clone(), value) {
            Some(_) => vecdeque_move_to_end!(self.order, key),
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
}

impl<K: std::hash::Hash + Eq, V> LRUCache<K, V> {
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    pub fn popitem(&mut self) -> Option<V> {
        match self.order.pop_front() {
            Some(x) => self.inner.remove(&x),
            None => None,
        }
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut c = 0usize;
        for _ in 0..n {
            if self.popitem().is_none() {
                break;
            }
            c += 1;
        }
        c
    }

    pub fn least_recently_used(&self) -> Option<&K> {
        self.order.front()
    }

    pub fn most_recently_used(&self) -> Option<&K> {
        self.order.back()
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        match self.inner.remove(key) {
            Some(val) => {
                let index = unsafe { self.order.iter().position(|x| x == key).unwrap_unchecked() };
                self.order.remove(index);
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
        self.order.clear();

        if !reuse {
            self.inner.shrink_to_fit();
            self.order.shrink_to_fit();
        }
    }

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.inner.keys()
    }

    pub fn sorted_keys(&self) -> std::collections::vec_deque::Iter<'_, K> {
        self.order.iter()
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
                vecdeque_move_to_end!(self.order, *key);
                Some(val)
            }
            None => None,
        }
    }

    pub fn fast_get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LRUCache<K, V> {
    pub fn setdefault(&mut self, key: K, default: V) -> pyo3::PyResult<V> {
        let exists = self.inner.get(&key);
        if exists.is_some() {
            return Ok(unsafe { exists.cloned().unwrap_unchecked() });
        }

        if self.maxsize > 0 && self.inner.len() >= self.maxsize {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        self.inner.insert(key.clone(), default.clone());
        self.order.push_back(key);

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(default)
    }
}

impl<K: PartialEq, V> PartialEq for LRUCache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.order == other.order
    }
}
impl<K: PartialEq, V> Eq for LRUCache<K, V> {}

/// This structure is used for keeping elements in TTLCache
#[derive(Clone)]
pub struct TTLValue<T: Clone> {
    pub value: T,
    pub expiration: time::Instant,
}

impl<T: Clone> TTLValue<T> {
    #[must_use]
    fn new(value: T, ttl: time::Duration) -> Self {
        Self {
            value,
            expiration: time::Instant::now() + ttl,
        }
    }

    pub fn expired(&self) -> bool {
        time::Instant::now() > self.expiration
    }
}

/// TTL Cache implementation (Time-to-live policy)
///
/// In simple terms, The TTL cache is one that evicts items that are older than a time-to-live.
pub struct TTLCache<K, V: Clone> {
    inner: AHashMap<K, TTLValue<V>>,
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
                inner: AHashMap::with_capacity(cap),
                order: VecDeque::with_capacity(cap),
                ttl: time::Duration::from_secs_f32(ttl),
                maxsize,
            };
        }

        Self {
            inner: AHashMap::new(),
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

        match self
            .inner
            .insert(key.clone(), TTLValue::new(value, self.ttl))
        {
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
            .insert(key.clone(), TTLValue::new(default.clone(), self.ttl));
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

    pub fn popitem(&mut self) -> Option<TTLValue<V>> {
        match self.order.pop_front() {
            Some(x) => self.inner.remove(&x),
            None => None,
        }
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut c = 0usize;
        for _ in 0..n {
            if self.popitem().is_none() {
                break;
            }
            c += 1;
        }
        c
    }

    pub fn remove(&mut self, key: &K) -> Option<TTLValue<V>> {
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

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, TTLValue<V>> {
        self.inner.keys()
    }

    pub fn sorted_keys(&self) -> std::collections::vec_deque::Iter<'_, K> {
        self.order.iter()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, TTLValue<V>> {
        self.inner.values()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, TTLValue<V>> {
        self.inner.iter()
    }

    pub fn get(&self, key: &K) -> Option<&TTLValue<V>> {
        self.inner.get(key).filter(|&val| !val.expired())
    }
}

impl<K: PartialEq + std::hash::Hash + Eq, V: Clone> PartialEq for TTLCache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.order == other.order
    }
}
impl<K: PartialEq + std::hash::Hash + Eq, V: Clone> Eq for TTLCache<K, V> {}

/// This structure is used for keeping elements in VTTLCache
#[derive(Clone)]
pub struct TTLValueOption<T: Clone> {
    pub value: T,
    pub expiration: Option<time::Instant>,
}

impl<T: Clone> TTLValueOption<T> {
    #[must_use]
    fn new(value: T, ttl: Option<f32>) -> Self {
        match ttl {
            Some(x) => Self {
                value,
                expiration: Some(time::Instant::now() + time::Duration::from_secs_f32(x)),
            },
            None => Self {
                value,
                expiration: None,
            },
        }
    }

    pub fn expired(&self) -> bool {
        self.expiration
            .is_some_and(|val| time::Instant::now() > val)
    }
}

/// VTTL Cache implementation (Time-to-live per-key policy)
///
/// Works like TTLCache, with this different that each key has own time-to-live value.
pub struct VTTLCache<K, V: Clone> {
    inner: AHashMap<K, TTLValueOption<V>>,
    order: Vec<K>,
    pub maxsize: usize,
}

impl<K, V: Clone> VTTLCache<K, V> {
    #[must_use]
    pub fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if maxsize > 0 && capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            return Self {
                inner: AHashMap::with_capacity(cap),
                order: Vec::with_capacity(cap),
                maxsize,
            };
        }

        Self {
            inner: AHashMap::new(),
            order: Vec::new(),
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

impl<K: std::hash::Hash + Eq + Clone + Ord, V: Clone> VTTLCache<K, V> {
    pub fn fast_insert(&mut self, key: K, value: V, ttl: Option<f32>) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self
            .inner
            .insert(key.clone(), TTLValueOption::new(value, ttl.or(None)))
        {
            Some(_) => (),
            None => self.order.push(key),
        }

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(())
    }

    pub fn insert(&mut self, key: K, value: V, ttl: Option<f32>) -> pyo3::PyResult<()> {
        if self.maxsize > 0 && self.inner.len() >= self.maxsize && self.inner.get(&key).is_none() {
            self.popitem();
        }

        let length = self.inner.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == self.inner.capacity();

        match self
            .inner
            .insert(key.clone(), TTLValueOption::new(value, ttl.or(None)))
        {
            Some(_) => (),
            None => self.order.push(key),
        }

        if length + 1 > 1 {
            // Sort from less to greater
            self.order.sort_by(|a, b| {
                let ap = self.inner.get(a).unwrap();
                let bp = self.inner.get(b).unwrap();

                if ap.expiration.is_none() && bp.expiration.is_none() {
                    return std::cmp::Ordering::Equal;
                }
                if bp.expiration.is_none() {
                    return std::cmp::Ordering::Greater;
                }
                if ap.expiration.is_none() {
                    return std::cmp::Ordering::Less;
                }
                bp.expiration.cmp(&ap.expiration)
            });
        }

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(())
    }

    pub fn update<T: IntoIterator<Item = pyo3::PyResult<(K, V)>>>(
        &mut self,
        iterable: T,
        ttl: Option<f32>,
    ) -> pyo3::PyResult<()> {
        for result in iterable {
            let (key, val) = result?;
            self.fast_insert(key, val, ttl)?;
        }

        if self.inner.len() > 1 {
            // Sort from less to greater
            self.order.sort_by(|a, b| {
                let ap = self.inner.get(a).unwrap();
                let bp = self.inner.get(b).unwrap();

                if ap.expiration.is_none() && bp.expiration.is_none() {
                    return std::cmp::Ordering::Equal;
                }
                if bp.expiration.is_none() {
                    return std::cmp::Ordering::Greater;
                }
                if ap.expiration.is_none() {
                    return std::cmp::Ordering::Less;
                }
                bp.expiration.cmp(&ap.expiration)
            });
        }

        Ok(())
    }

    pub fn setdefault(&mut self, key: K, default: V, ttl: Option<f32>) -> pyo3::PyResult<V> {
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

        self.inner.insert(
            key.clone(),
            TTLValueOption::new(default.clone(), ttl.or(None)),
        );
        self.order.push(key);

        if length + 1 > 1 {
            // Sort from less to greater
            self.order.sort_by(|a, b| {
                let ap = self.inner.get(a).unwrap();
                let bp = self.inner.get(b).unwrap();

                if ap.expiration.is_none() && bp.expiration.is_none() {
                    return std::cmp::Ordering::Equal;
                }
                if bp.expiration.is_none() {
                    return std::cmp::Ordering::Greater;
                }
                if ap.expiration.is_none() {
                    return std::cmp::Ordering::Less;
                }
                bp.expiration.cmp(&ap.expiration)
            });
        }

        if time_to_shrink {
            self.inner.shrink_to_fit();
        }

        Ok(default)
    }
}

impl<K: std::hash::Hash + Eq + Ord, V: Clone> VTTLCache<K, V> {
    pub fn expire(&mut self) {
        while let Some(key) = self.order.last() {
            if !self.inner[key].expired() {
                break;
            }

            self.inner.remove(key);
            self.order.pop();
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    pub fn popitem(&mut self) -> Option<TTLValueOption<V>> {
        match self.order.pop() {
            Some(key) => self.inner.remove(&key),
            None => None,
        }
    }

    pub fn drain(&mut self, n: usize) -> usize {
        let mut c = 0usize;
        for _ in 0..n {
            if self.popitem().is_none() {
                break;
            }
            c += 1;
        }
        c
    }

    pub fn remove(&mut self, key: &K) -> Option<TTLValueOption<V>> {
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

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, TTLValueOption<V>> {
        self.inner.keys()
    }

    pub fn sorted_keys(&self) -> std::slice::Iter<'_, K> {
        self.order.iter()
    }

    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, TTLValueOption<V>> {
        self.inner.values()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, TTLValueOption<V>> {
        self.inner.iter()
    }

    pub fn get(&self, key: &K) -> Option<&TTLValueOption<V>> {
        self.inner.get(key).filter(|&val| !val.expired())
    }
}

impl<K: PartialEq + std::hash::Hash + Eq, V: Clone> PartialEq for VTTLCache<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.maxsize == other.maxsize && self.order == other.order
    }
}
impl<K: PartialEq + std::hash::Hash + Eq, V: Clone> Eq for VTTLCache<K, V> {}
