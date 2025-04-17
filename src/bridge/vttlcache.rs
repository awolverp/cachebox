use crate::common::Entry;
use crate::common::ObservedIterator;
use crate::common::PreHashObject;
use crate::common::TimeToLivePair;

#[pyo3::pyclass(module = "cachebox._core", frozen)]
pub struct VTTLCache {
    raw: crate::mutex::Mutex<crate::policies::vttl::VTTLPolicy>,
}

#[allow(non_camel_case_types)]
#[pyo3::pyclass(module = "cachebox._core")]
pub struct vttlcache_items {
    pub ptr: ObservedIterator,
    pub iter: crate::mutex::Mutex<crate::policies::vttl::VTTLIterator>,
    pub now: std::time::SystemTime,
}

#[pyo3::pymethods]
impl VTTLCache {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0))]
    fn __new__(maxsize: usize, capacity: usize) -> pyo3::PyResult<Self> {
        let raw = crate::policies::vttl::VTTLPolicy::new(maxsize, capacity)?;

        let self_ = Self {
            raw: crate::mutex::Mutex::new(raw),
        };
        Ok(self_)
    }

    fn _state(&self) -> u16 {
        self.raw.lock().observed.get()
    }

    fn maxsize(&self) -> usize {
        self.raw.lock().maxsize()
    }

    fn capacity(&self) -> usize {
        self.raw.lock().capacity()
    }

    fn __len__(&self) -> usize {
        self.raw.lock().real_len()
    }

    fn __sizeof__(&self) -> usize {
        let lock = self.raw.lock();

        lock.capacity()
            * (std::mem::size_of::<PreHashObject>() + std::mem::size_of::<pyo3::ffi::PyObject>())
    }

    fn __contains__(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<bool> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.lookup(py, &key)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn is_empty(&self) -> bool {
        self.raw.lock().is_empty()
    }

    fn is_full(&self) -> bool {
        self.raw.lock().is_full()
    }

    #[pyo3(signature=(key, value, ttl=None))]
    fn insert(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.entry_with_slot(py, &key)? {
            Entry::Occupied(entry) => Ok(Some(entry.update(value, ttl)?)),
            Entry::Absent(entry) => {
                entry.insert(key, value, ttl)?;
                Ok(None)
            }
        }
    }

    fn get(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<super::TTLPair> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.lookup(py, &key)? {
            Some(val) => Ok(super::TTLPair::clone_from_pair(py, val)),
            None => Err(pyo3::PyErr::new::<super::CoreKeyError, _>(key.obj)),
        }
    }

    #[pyo3(signature=(iterable, ttl=None))]
    fn update(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
        iterable: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<()> {
        if slf.as_ptr() == iterable.as_ptr() {
            return Ok(());
        }

        let mut lock = slf.raw.lock();
        lock.extend(py, iterable, ttl)
    }

    fn __richcmp__(
        slf: pyo3::PyRef<'_, Self>,
        other: pyo3::PyObject,
        op: pyo3::class::basic::CompareOp,
    ) -> pyo3::PyResult<bool> {
        let other = other.extract::<pyo3::PyRef<'_, Self>>(slf.py())?;

        match op {
            pyo3::class::basic::CompareOp::Eq => {
                if slf.as_ptr() == other.as_ptr() {
                    return Ok(true);
                }

                let mut t1 = slf.raw.lock();
                let mut t2 = other.raw.lock();
                t1.equal(slf.py(), &mut t2)
            }
            pyo3::class::basic::CompareOp::Ne => {
                if slf.as_ptr() == other.as_ptr() {
                    return Ok(false);
                }

                let mut t1 = slf.raw.lock();
                let mut t2 = other.raw.lock();
                t1.equal(slf.py(), &mut t2).map(|r| !r)
            }
            _ => Err(pyo3::PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "only '==' or '!=' are supported",
            )),
        }
    }

    fn remove(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<super::TTLPair> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.entry(py, &key)? {
            Entry::Occupied(entry) => {
                let val = entry.remove();
                Ok(super::TTLPair::from(val))
            }
            Entry::Absent(_) => Err(pyo3::PyErr::new::<super::CoreKeyError, _>(key.obj)),
        }
    }

    fn popitem(&self) -> pyo3::PyResult<super::TTLPair> {
        let mut lock = self.raw.lock();

        match lock.popitem() {
            Some(val) => Ok(super::TTLPair::from(val)),
            None => Err(pyo3::PyErr::new::<super::CoreKeyError, _>(())),
        }
    }

    fn clear(&self, reuse: bool) {
        let mut lock = self.raw.lock();
        lock.clear();

        if !reuse {
            lock.shrink_to_fit();
        }
    }

    fn shrink_to_fit(&self) {
        let mut lock = self.raw.lock();
        lock.shrink_to_fit();
    }

    #[pyo3(signature=(key, default, ttl=None))]
    fn setdefault(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.entry(py, &key)? {
            Entry::Occupied(entry) => unsafe {
                let val = entry.into_value();
                Ok(val.as_ref().value.clone_ref(py))
            },
            Entry::Absent(entry) => {
                entry.insert(key, default.clone_ref(py), ttl)?;
                Ok(default)
            }
        }
    }

    fn items(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyResult<pyo3::Py<vttlcache_items>> {
        let mut lock = slf.raw.lock();
        let state = lock.observed.get();
        let iter = lock.iter();

        let result = vttlcache_items {
            ptr: ObservedIterator::new(slf.as_ptr(), state),
            iter: crate::mutex::Mutex::new(iter),
            now: std::time::SystemTime::now(),
        };

        pyo3::Py::new(slf.py(), result)
    }

    fn expire(&self) {
        let mut lock = self.raw.lock();
        lock.expire();
        lock.shrink_to_fit();
    }

    fn __getnewargs__(&self) -> (usize,) {
        (0,)
    }

    fn __getstate__(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        let mut lock = self.raw.lock();
        lock.expire();

        let state = unsafe {
            let list = pyo3::ffi::PyList_New(0);
            if list.is_null() {
                return Err(pyo3::PyErr::fetch(py));
            }

            for ptr in lock.iter() {
                let node = ptr.as_ref();

                let ttlobject = pyo3::ffi::PyLong_FromDouble(node.expire_at.map_or(0.0, |x| {
                    x.duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64()
                }));

                if ttlobject.is_null() {
                    pyo3::ffi::Py_DECREF(list);
                    return Err(pyo3::PyErr::fetch(py));
                }

                let tp = tuple!(
                    py,
                    3,
                    0 => node.key.obj.clone_ref(py).as_ptr(),
                    1 => node.value.clone_ref(py).as_ptr(),
                    2 => ttlobject,
                );

                if let Err(x) = tp {
                    pyo3::ffi::Py_DECREF(list);
                    return Err(x);
                }

                if pyo3::ffi::PyList_Append(list, tp.unwrap_unchecked()) == -1 {
                    pyo3::ffi::Py_DECREF(list);
                    return Err(pyo3::PyErr::fetch(py));
                }
            }

            let maxsize = pyo3::ffi::PyLong_FromSize_t(lock.maxsize());
            let capacity = pyo3::ffi::PyLong_FromSize_t(lock.capacity());

            tuple!(
                py,
                3,
                0 => maxsize,
                1 => list,
                2 => capacity,
            )?
        };

        Ok(unsafe { pyo3::Py::from_owned_ptr(py, state) })
    }

    pub fn __setstate__(&self, py: pyo3::Python<'_>, state: pyo3::PyObject) -> pyo3::PyResult<()> {
        let mut lock = self.raw.lock();
        lock.from_pickle(py, state.as_ptr())
    }

    pub fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        for node in self.raw.lock().iter() {
            let value = unsafe { node.as_ref() };

            visit.call(&value.key.obj)?;
            visit.call(&value.value)?;
        }
        Ok(())
    }

    pub fn __clear__(&self) {
        let mut lock = self.raw.lock();
        lock.clear()
    }
}

#[pyo3::pymethods]
impl vttlcache_items {
    fn __iter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> {
        slf
    }

    #[allow(unused_mut)]
    fn __next__(mut slf: pyo3::PyRefMut<'_, Self>) -> pyo3::PyResult<super::TTLPair> {
        let mut iter = slf.iter.lock();

        slf.ptr.proceed(slf.py())?;

        let mut element: std::ptr::NonNull<TimeToLivePair>;
        loop {
            element = {
                if let Some(x) = iter.next() {
                    x
                } else {
                    return Err(pyo3::PyErr::new::<pyo3::exceptions::PyStopIteration, _>(()));
                }
            };

            if unsafe { !element.as_ref().is_expired(slf.now) } {
                break;
            }
        }

        Ok(super::TTLPair::clone_from_pair(slf.py(), unsafe {
            element.as_ref()
        }))
    }
}
