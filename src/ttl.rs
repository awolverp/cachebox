use pyo3::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;
use std::time;

use crate::base;

#[pyclass(extends=base::BaseCacheImpl, module="cachebox._cachebox", subclass)]
pub struct TTLCacheNoDefault {
    inner: RwLock<HashMap<isize, base::TTLKeyValuePair>>,

    // Will sort by expiration time
    order: Vec<isize>,
    maxsize: usize,
}

impl TTLCacheNoDefault {
    fn new(maxsize: usize, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            TTLCacheNoDefault {
                inner: RwLock::new(HashMap::with_capacity(cap)),
                order: Vec::with_capacity(cap),
                maxsize,
            }
        } else {
            TTLCacheNoDefault {
                inner: RwLock::new(HashMap::new()),
                order: Vec::new(),
                maxsize,
            }
        }
    }

    fn cache_expire(&mut self) {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        while let Some(key) = self.order.last() {
            if !write[key].is_expired() {
                break;
            }

            write.remove(key);
            self.order.pop();
        }
    }

    fn cache_popitem_without_expire(&mut self) -> Option<base::TTLKeyValuePair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        match self.order.pop() {
            Some(key) => write.remove(&key),
            None => None,
        }
    }

    fn cache_popitem(&mut self) -> Option<base::TTLKeyValuePair> {
        self.cache_expire();

        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        match self.order.pop() {
            Some(key) => write.remove(&key),
            None => None,
        }
    }

    fn cache_setitem_without_sort(
        &mut self,
        hash: isize,
        key: Py<PyAny>,
        value: Py<PyAny>,
        expire: Option<time::Instant>,
    ) -> PyResult<()> {
        self.cache_expire();

        if self.maxsize > 0 {
            let read = self.inner.read().expect("RwLock is poisoned (read)");
            let length = read.len() + 1;

            if length > self.maxsize && read.get(&hash).is_none() {
                drop(read);

                for _ in 0..(length - self.maxsize) {
                    if self.cache_popitem_without_expire().is_none() {
                        break;
                    }
                }
            }
        }

        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        let length = write.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == write.capacity();

        match write.insert(hash, base::TTLKeyValuePair { key, value, expire }) {
            Some(_) => (),
            None => self.order.push(hash),
        }

        if time_to_shrink {
            write.shrink_to_fit();
        }

        Ok(())
    }

    fn cache_setitem(
        &mut self,
        hash: isize,
        key: Py<PyAny>,
        value: Py<PyAny>,
        expire: Option<time::Instant>,
    ) -> PyResult<()> {
        self.cache_expire();

        if self.maxsize > 0 {
            let read = self.inner.read().expect("RwLock is poisoned (read)");
            let length = read.len() + 1;

            if length > self.maxsize && read.get(&hash).is_none() {
                drop(read);

                for _ in 0..(length - self.maxsize) {
                    if self.cache_popitem_without_expire().is_none() {
                        break;
                    }
                }
            }
        }

        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        let length = write.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == write.capacity();

        match write.insert(hash, base::TTLKeyValuePair { key, value, expire }) {
            Some(_) => (),
            None => self.order.push(hash),
        }

        if length + 1 > 1 {
            // Sort from less to greater
            self.order.sort_unstable_by(|a, b| {
                let ap = write.get(a).unwrap();
                let bp = write.get(b).unwrap();

                if ap.expire.is_none() && bp.expire.is_none() {
                    return std::cmp::Ordering::Equal;
                }
                if bp.expire.is_none() {
                    return std::cmp::Ordering::Greater;
                }
                if ap.expire.is_none() {
                    return std::cmp::Ordering::Less;
                }
                bp.expire.cmp(&ap.expire)
            });
        }

        if time_to_shrink {
            write.shrink_to_fit();
        }

        Ok(())
    }

    fn cache_remove(&mut self, hash: &isize) -> Option<base::TTLKeyValuePair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        match write.remove(hash) {
            Some(v) => {
                let index = self.order.iter().position(|x| *x == *hash).unwrap();
                self.order.remove(index);

                if v.is_expired() {
                    return None;
                }
                Some(v)
            }
            None => None,
        }
    }

    fn cache_len(&mut self) -> usize {
        self.cache_expire();
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.len()
    }

    fn cache_contains(&self, hash: &isize) -> bool {
        let read = self.inner.read().expect("RwLock is poisoned (read)");

        match read.get(hash) {
            Some(v) => !v.is_expired(),
            None => false,
        }
    }

    fn cache_clear(&mut self, reuse: bool) {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        write.clear();
        self.order.clear();

        if !reuse {
            write.shrink_to_fit();
            self.order.shrink_to_fit();
        }
    }

    fn cache_sizeof(&self) -> usize {
        let read = self.inner.read().expect("RwLock is poisoned (read)");

        let cap = read.capacity();

        cap * base::ISIZE_MEMORY_SIZE
            + cap * base::INSTANT_MEMORY_SIZE
            + self.order.capacity() * base::ISIZE_MEMORY_SIZE
            + base::ISIZE_MEMORY_SIZE
    }

    fn cache_keys(&mut self) -> Vec<Py<PyAny>> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        self.order
            .iter()
            .map(|x| read.get(x).unwrap().key.clone())
            .collect()
    }

    fn cache_values(&mut self) -> Vec<Py<PyAny>> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        self.order
            .iter()
            .map(|x| read.get(x).unwrap().value.clone())
            .collect()
    }

    fn cache_items(&mut self) -> Vec<(Py<PyAny>, Py<PyAny>)> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        self.order
            .iter()
            .map(|x| {
                let y = read.get(x).unwrap();
                (y.key.clone(), y.value.clone())
            })
            .collect()
    }

    fn cache_equal(&self, other: &Self) -> bool {
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        let other_read = other.inner.read().expect("RwLock is poisoned (read)");

        read.len() == other_read.len() && read.keys().all(|x| other_read.contains_key(x))
    }

    fn cache_update_from_pydict(
        &mut self,
        other: &pyo3::types::PyDict,
        expire: Option<time::Instant>,
    ) -> PyResult<()> {
        for i in other.items() {
            let items: (&PyAny, &PyAny) = i.extract()?;
            self.cache_setitem_without_sort(
                items.0.hash()?,
                items.0.into(),
                items.1.into(),
                expire,
            )?;
        }

        let write = self.inner.write().expect("RwLock is poisoned (write)");

        // Sort from less to greater
        self.order.sort_unstable_by(|a, b| {
            let ap = write.get(a).unwrap();
            let bp = write.get(b).unwrap();

            if ap.expire.is_none() && bp.expire.is_none() {
                return std::cmp::Ordering::Equal;
            }
            if bp.expire.is_none() {
                return std::cmp::Ordering::Greater;
            }
            if ap.expire.is_none() {
                return std::cmp::Ordering::Less;
            }
            bp.expire.cmp(&ap.expire)
        });

        Ok(())
    }

    fn cache_update_from_pyobject(
        &mut self,
        other: &pyo3::types::PyIterator,
        expire: Option<time::Instant>,
    ) -> PyResult<()> {
        for i in other {
            let items: (&PyAny, &PyAny) = i?.extract()?;
            self.cache_setitem_without_sort(
                items.0.hash()?,
                items.0.into(),
                items.1.into(),
                expire,
            )?;
        }

        let write = self.inner.write().expect("RwLock is poisoned (write)");

        // Sort from less to greater
        self.order.sort_unstable_by(|a, b| {
            let ap = write.get(a).unwrap();
            let bp = write.get(b).unwrap();

            if ap.expire.is_none() && bp.expire.is_none() {
                return std::cmp::Ordering::Equal;
            }
            if bp.expire.is_none() {
                return std::cmp::Ordering::Greater;
            }
            if ap.expire.is_none() {
                return std::cmp::Ordering::Less;
            }
            bp.expire.cmp(&ap.expire)
        });

        Ok(())
    }
}

#[pymethods]
impl TTLCacheNoDefault {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0))]
    pub fn __new__(maxsize: usize, capacity: usize) -> (Self, base::BaseCacheImpl) {
        (
            TTLCacheNoDefault::new(maxsize, capacity),
            base::BaseCacheImpl {},
        )
    }

    pub fn __setitem__(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_setitem(hash, key, value, None) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub fn __delitem__(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_remove(&hash) {
            Some(_) => Ok(()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    pub fn __contains__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<bool> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        Ok(self.cache_contains(&hash))
    }

    pub fn __len__(&mut self) -> PyResult<usize> {
        Ok(self.cache_len())
    }

    pub fn __repr__(&mut self) -> PyResult<String> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        Ok(format!(
            "<cachebox._cachebox.TTLCacheNoDefault len={} maxsize={} capacity={}>",
            read.len(),
            self.maxsize,
            read.capacity()
        ))
    }

    pub fn __sizeof__(&self) -> PyResult<usize> {
        Ok(self.cache_sizeof())
    }

    pub fn __richcmp__(&self, other: &Self, op: pyo3::class::basic::CompareOp) -> PyResult<bool> {
        match op {
            pyo3::class::basic::CompareOp::Eq => Ok(self.cache_equal(other)),
            pyo3::class::basic::CompareOp::Ne => Ok(!self.cache_equal(other)),
            _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "only == and != operations are supported",
            )),
        }
    }

    #[pyo3(signature=(key, value, ttl=None))]
    pub fn insert(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        value: Py<PyAny>,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match ttl {
            Some(seconds) => {
                if seconds <= 0.0 {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "ttl parameter cannot be zero or negative; if you want set no expire time, pass None."
                    ));
                }

                let duration = time::Duration::from_millis((seconds * 1000.0) as u64);

                match self.cache_setitem(hash, key, value, Some(time::Instant::now() + duration)) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err),
                }
            }
            None => match self.cache_setitem(hash, key, value, None) {
                Ok(_) => Ok(()),
                Err(err) => Err(err),
            },
        }
    }

    pub fn delete(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_remove(&hash) {
            Some(_) => Ok(()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    pub fn getmaxsize(&self) -> PyResult<usize> {
        Ok(self.maxsize)
    }

    pub fn keys(&mut self) -> PyResult<Vec<Py<PyAny>>> {
        Ok(self.cache_keys())
    }

    pub fn values(&mut self) -> PyResult<Vec<Py<PyAny>>> {
        Ok(self.cache_values())
    }

    pub fn items(&mut self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>)>> {
        Ok(self.cache_items())
    }

    #[pyo3(signature=(key, default=None))]
    pub fn pop(
        &mut self,
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

        match self.cache_remove(&hash) {
            Some(v) => Ok(Some(v.value)),
            None => Ok(default),
        }
    }

    #[pyo3(signature=(key, default=None, ttl=None))]
    pub fn setdefault(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
        ttl: Option<f32>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        if let Some(v) = read.get(&hash) {
            Ok(Some(v.value.clone()))
        } else {
            drop(read);

            let defaultvalue: Py<PyAny>;
            if let Some(v) = default {
                defaultvalue = v;
            } else {
                defaultvalue = py.None();
            }

            match ttl {
                Some(seconds) => {
                    if seconds <= 0.0 {
                        drop(defaultvalue);
                        return Err(pyo3::exceptions::PyValueError::new_err(
                            "ttl parameter cannot be zero or negative; if you want set no expire time, pass None."
                        ));
                    }

                    let duration = time::Duration::from_millis((seconds * 1000.0) as u64);

                    match self.cache_setitem(
                        hash,
                        key,
                        defaultvalue.clone(),
                        Some(time::Instant::now() + duration),
                    ) {
                        Ok(_) => Ok(Some(defaultvalue)),
                        Err(err) => {
                            drop(defaultvalue);
                            Err(err)
                        }
                    }
                }
                None => match self.cache_setitem(hash, key, defaultvalue.clone(), None) {
                    Ok(_) => Ok(Some(defaultvalue)),
                    Err(err) => {
                        drop(defaultvalue);
                        Err(err)
                    }
                },
            }
        }
    }

    #[pyo3(signature=(iterable, ttl=None))]
    pub fn update(
        &mut self,
        py: Python<'_>,
        iterable: Py<PyAny>,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        let obj = iterable.as_ref(py);

        let dur: Option<time::Instant> = match ttl {
            Some(seconds) => {
                if seconds <= 0.0 {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "ttl parameter cannot be zero or negative; if you want set no expire time, pass None."
                    ));
                }

                let duration = time::Duration::from_millis((seconds * 1000.0) as u64);
                Some(time::Instant::now() + duration)
            }
            None => None,
        };

        if obj.is_instance_of::<pyo3::types::PyDict>() {
            return self.cache_update_from_pydict(obj.extract()?, dur);
        }

        let i = obj.iter()?;
        self.cache_update_from_pyobject(i, dur)
    }

    #[pyo3(signature=(*, reuse=false))]
    pub fn clear(&mut self, reuse: bool) -> PyResult<()> {
        self.cache_clear(reuse);
        Ok(())
    }

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
                if v.is_expired() {
                    Err(pyo3::exceptions::PyKeyError::new_err(format!("{}", key)))
                } else {
                    Ok(v.value.clone())
                }
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
                if v.is_expired() {
                    Ok(default)
                } else {
                    Ok(Some(v.value.clone()))
                }
            }
            None => Ok(default),
        }
    }

    pub fn popitem(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
        match self.cache_popitem() {
            None => Err(pyo3::exceptions::PyKeyError::new_err("cache is empty")),
            Some(v) => Ok((v.key, v.value)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn get_with_expire(
        &self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<(Option<Py<PyAny>>, f32)> {
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
                if v.is_expired() {
                    Ok((default, 0.0))
                } else {
                    match v.expire {
                        Some(ttl) => {
                            let dur = ttl - time::Instant::now();
                            Ok((Some(v.value.clone()), dur.as_secs_f32()))
                        }
                        None => Ok((Some(v.value.clone()), 0.0)),
                    }
                }
            }
            None => Ok((default, 0.0)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn pop_with_expire(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<(Option<Py<PyAny>>, f32)> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_remove(&hash) {
            Some(v) => {
                if v.is_expired() {
                    Ok((default, 0.0))
                } else {
                    match v.expire {
                        Some(ttl) => {
                            let dur = ttl - time::Instant::now();
                            Ok((Some(v.value.clone()), dur.as_secs_f32()))
                        }
                        None => Ok((Some(v.value.clone()), 0.0)),
                    }
                }
            }
            None => Ok((default, 0.0)),
        }
    }

    pub fn popitem_with_expire(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>, f32)> {
        match self.cache_popitem() {
            None => Err(pyo3::exceptions::PyKeyError::new_err("cache is empty")),
            Some(v) => match v.expire {
                Some(ttl) => {
                    let dur = ttl - time::Instant::now();
                    Ok((v.key, v.value, dur.as_secs_f32()))
                }
                None => Ok((v.key, v.value, 0.0)),
            },
        }
    }

    #[pyo3(signature=(*, reuse=false))]
    pub fn expire(&mut self, reuse: bool) -> PyResult<()> {
        self.cache_expire();

        if !reuse {
            let mut write = self.inner.write().expect("RwLock is poisoned (write)");
            write.shrink_to_fit();
            self.order.shrink_to_fit();
        }

        Ok(())
    }
}

#[pyclass(extends=base::BaseCacheImpl, module="cachebox._cachebox", subclass)]
pub struct TTLCache {
    inner: RwLock<HashMap<isize, base::TTLKeyValuePair>>,

    // Will sort by expiration time
    order: VecDeque<isize>,
    maxsize: usize,
    ttl: time::Duration,
}

impl TTLCache {
    fn new(maxsize: usize, ttl: time::Duration, capacity: usize) -> Self {
        if capacity > 0 {
            let cap = if capacity <= maxsize {
                capacity
            } else {
                maxsize
            };

            TTLCache {
                inner: RwLock::new(HashMap::with_capacity(cap)),
                order: VecDeque::with_capacity(cap),
                maxsize,
                ttl,
            }
        } else {
            TTLCache {
                inner: RwLock::new(HashMap::new()),
                order: VecDeque::new(),
                maxsize,
                ttl,
            }
        }
    }

    fn cache_expire(&mut self) {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        while let Some(key) = self.order.front() {
            if !write[key].is_expired() {
                break;
            }

            write.remove(key);
            self.order.pop_front();
        }
    }

    fn cache_popitem_without_expire(&mut self) -> Option<base::TTLKeyValuePair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        match self.order.pop_front() {
            Some(key) => write.remove(&key),
            None => None,
        }
    }

    fn cache_popitem(&mut self) -> Option<base::TTLKeyValuePair> {
        self.cache_expire();

        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        match self.order.pop_front() {
            Some(key) => write.remove(&key),
            None => None,
        }
    }

    fn cache_setitem(&mut self, hash: isize, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()> {
        self.cache_expire();

        if self.maxsize > 0 {
            let read = self.inner.read().expect("RwLock is poisoned (read)");
            let length = read.len() + 1;

            if length > self.maxsize && read.get(&hash).is_none() {
                drop(read);

                for _ in 0..(length - self.maxsize) {
                    if self.cache_popitem_without_expire().is_none() {
                        break;
                    }
                }
            }
        }

        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        let length = write.len();
        let time_to_shrink = ((length + 1) == self.maxsize) && length == write.capacity();

        let dur = time::Instant::now() + self.ttl;
        match write.insert(
            hash,
            base::TTLKeyValuePair {
                key,
                value,
                expire: Some(dur),
            },
        ) {
            Some(_) => (),
            None => self.order.push_back(hash),
        }

        if time_to_shrink {
            write.shrink_to_fit();
        }

        Ok(())
    }

    fn cache_remove(&mut self, hash: &isize) -> Option<base::TTLKeyValuePair> {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");

        match write.remove(hash) {
            Some(v) => {
                let index = self.order.iter().position(|x| *x == *hash).unwrap();
                self.order.remove(index);

                if v.is_expired() {
                    return None;
                }
                Some(v)
            }
            None => None,
        }
    }

    fn cache_len(&mut self) -> usize {
        self.cache_expire();
        let read = self.inner.read().expect("RwLock is poisoned (read)");
        read.len()
    }

    fn cache_contains(&self, hash: &isize) -> bool {
        let read = self.inner.read().expect("RwLock is poisoned (read)");

        match read.get(hash) {
            Some(v) => !v.is_expired(),
            None => false,
        }
    }

    fn cache_clear(&mut self, reuse: bool) {
        let mut write = self.inner.write().expect("RwLock is poisoned (write)");
        write.clear();
        self.order.clear();

        if !reuse {
            write.shrink_to_fit();
            self.order.shrink_to_fit();
        }
    }

    fn cache_sizeof(&self) -> usize {
        let read = self.inner.read().expect("RwLock is poisoned (read)");

        let cap = read.capacity();

        cap * base::ISIZE_MEMORY_SIZE
            + cap * base::INSTANT_MEMORY_SIZE
            + self.order.capacity() * base::ISIZE_MEMORY_SIZE
            + base::ISIZE_MEMORY_SIZE
    }

    fn cache_keys(&mut self) -> Vec<Py<PyAny>> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        self.order
            .iter()
            .map(|x| read.get(x).unwrap().key.clone())
            .collect()
    }

    fn cache_values(&mut self) -> Vec<Py<PyAny>> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        self.order
            .iter()
            .map(|x| read.get(x).unwrap().value.clone())
            .collect()
    }

    fn cache_items(&mut self) -> Vec<(Py<PyAny>, Py<PyAny>)> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        self.order
            .iter()
            .map(|x| {
                let y = read.get(x).unwrap();
                (y.key.clone(), y.value.clone())
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
impl TTLCache {
    #[new]
    #[pyo3(signature=(maxsize, ttl, *, capacity=0))]
    pub fn __new__(
        maxsize: usize,
        ttl: f32,
        capacity: usize,
    ) -> PyResult<(Self, base::BaseCacheImpl)> {
        if ttl <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err("ttl parameter cannot be zero or negative; if you do not want ttl, try other caches."));
        }

        Ok((
            TTLCache::new(
                maxsize,
                time::Duration::from_millis((ttl * 1000.0) as u64),
                capacity,
            ),
            base::BaseCacheImpl {},
        ))
    }

    pub fn __setitem__(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_setitem(hash, key, value) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub fn __delitem__(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_remove(&hash) {
            Some(_) => Ok(()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    pub fn __contains__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<bool> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        Ok(self.cache_contains(&hash))
    }

    pub fn __len__(&mut self) -> PyResult<usize> {
        Ok(self.cache_len())
    }

    pub fn __repr__(&mut self) -> PyResult<String> {
        self.cache_expire();

        let read = self.inner.read().expect("RwLock is poisoned (read)");
        Ok(format!(
            "<cachebox._cachebox.TTLCacheNoDefault len={} maxsize={} capacity={}>",
            read.len(),
            self.maxsize,
            read.capacity()
        ))
    }

    pub fn __sizeof__(&self) -> PyResult<usize> {
        Ok(self.cache_sizeof())
    }

    pub fn __richcmp__(&self, other: &Self, op: pyo3::class::basic::CompareOp) -> PyResult<bool> {
        match op {
            pyo3::class::basic::CompareOp::Eq => Ok(self.cache_equal(other)),
            pyo3::class::basic::CompareOp::Ne => Ok(!self.cache_equal(other)),
            _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "only == and != operations are supported",
            )),
        }
    }

    #[pyo3(signature=(key, value))]
    pub fn insert(&mut self, py: Python<'_>, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_setitem(hash, key, value) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub fn delete(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_remove(&hash) {
            Some(_) => Ok(()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key.to_string())),
        }
    }

    pub fn getmaxsize(&self) -> PyResult<usize> {
        Ok(self.maxsize)
    }

    pub fn getttl(&self) -> PyResult<f32> {
        Ok(self.ttl.as_secs_f32())
    }

    pub fn keys(&mut self) -> PyResult<Vec<Py<PyAny>>> {
        Ok(self.cache_keys())
    }

    pub fn values(&mut self) -> PyResult<Vec<Py<PyAny>>> {
        Ok(self.cache_values())
    }

    pub fn items(&mut self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>)>> {
        Ok(self.cache_items())
    }

    #[pyo3(signature=(key, default=None))]
    pub fn pop(
        &mut self,
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

        match self.cache_remove(&hash) {
            Some(v) => Ok(Some(v.value)),
            None => Ok(default),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn setdefault(
        &mut self,
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
        if let Some(v) = read.get(&hash) {
            Ok(Some(v.value.clone()))
        } else {
            drop(read);

            let defaultvalue: Py<PyAny>;
            if let Some(v) = default {
                defaultvalue = v;
            } else {
                defaultvalue = py.None();
            }

            match self.cache_setitem(hash, key, defaultvalue.clone()) {
                Ok(_) => Ok(Some(defaultvalue)),
                Err(err) => {
                    drop(defaultvalue);
                    Err(err)
                }
            }
        }
    }

    #[pyo3(signature=(iterable))]
    pub fn update(&mut self, py: Python<'_>, iterable: Py<PyAny>) -> PyResult<()> {
        let obj = iterable.as_ref(py);

        if obj.is_instance_of::<pyo3::types::PyDict>() {
            return self.cache_update_from_pydict(obj.extract()?);
        }

        let i = obj.iter()?;
        self.cache_update_from_pyobject(i)
    }

    #[pyo3(signature=(*, reuse=false))]
    pub fn clear(&mut self, reuse: bool) -> PyResult<()> {
        self.cache_clear(reuse);
        Ok(())
    }

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
                if v.is_expired() {
                    Err(pyo3::exceptions::PyKeyError::new_err(format!("{}", key)))
                } else {
                    Ok(v.value.clone())
                }
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
                if v.is_expired() {
                    Ok(default)
                } else {
                    Ok(Some(v.value.clone()))
                }
            }
            None => Ok(default),
        }
    }

    pub fn popitem(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
        match self.cache_popitem() {
            None => Err(pyo3::exceptions::PyKeyError::new_err("cache is empty")),
            Some(v) => Ok((v.key, v.value)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn get_with_expire(
        &self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<(Option<Py<PyAny>>, f32)> {
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
                if v.is_expired() {
                    Ok((default, 0.0))
                } else {
                    match v.expire {
                        Some(ttl) => {
                            let dur = ttl - time::Instant::now();
                            Ok((Some(v.value.clone()), dur.as_secs_f32()))
                        }
                        None => unreachable!(),
                    }
                }
            }
            None => Ok((default, 0.0)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    pub fn pop_with_expire(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<(Option<Py<PyAny>>, f32)> {
        let _ref = key.as_ref(py);
        let hash = match _ref.hash() {
            Ok(h) => h,
            Err(err) => {
                return Err(err);
            }
        };

        match self.cache_remove(&hash) {
            Some(v) => {
                if v.is_expired() {
                    Ok((default, 0.0))
                } else {
                    match v.expire {
                        Some(ttl) => {
                            let dur = ttl - time::Instant::now();
                            Ok((Some(v.value.clone()), dur.as_secs_f32()))
                        }
                        None => unreachable!(),
                    }
                }
            }
            None => Ok((default, 0.0)),
        }
    }

    pub fn popitem_with_expire(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>, f32)> {
        match self.cache_popitem() {
            Some(v) => match v.expire {
                Some(ttl) => {
                    let dur = ttl - time::Instant::now();
                    Ok((v.key, v.value, dur.as_secs_f32()))
                }
                None => unreachable!(),
            },
            None => Err(pyo3::exceptions::PyKeyError::new_err("cache is empty")),
        }
    }

    #[pyo3(signature=(*, reuse=false))]
    pub fn expire(&mut self, reuse: bool) -> PyResult<()> {
        self.cache_expire();

        if !reuse {
            let mut write = self.inner.write().expect("RwLock is poisoned (write)");
            write.shrink_to_fit();
            self.order.shrink_to_fit();
        }

        Ok(())
    }
}
