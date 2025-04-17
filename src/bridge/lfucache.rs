use crate::common::Entry;
use crate::common::ObservedIterator;
use crate::common::PreHashObject;

#[pyo3::pyclass(module = "cachebox._core", frozen)]
pub struct LFUCache {
    raw: crate::mutex::Mutex<crate::policies::lfu::LFUPolicy>,
}

#[allow(non_camel_case_types)]
#[pyo3::pyclass(module = "cachebox._core")]
pub struct lfucache_items {
    pub ptr: ObservedIterator,
    pub iter: crate::mutex::Mutex<crate::policies::lfu::LFUIterator>,
}

#[pyo3::pymethods]
impl LFUCache {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0))]
    fn __new__(maxsize: usize, capacity: usize) -> pyo3::PyResult<Self> {
        let raw = crate::policies::lfu::LFUPolicy::new(maxsize, capacity)?;

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
        self.raw.lock().len()
    }

    fn __sizeof__(&self) -> usize {
        let lock = self.raw.lock();

        lock.capacity()
            * (std::mem::size_of::<PreHashObject>() + std::mem::size_of::<pyo3::ffi::PyObject>())
    }

    fn __contains__(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<bool> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

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

    #[pyo3(signature=(key, value, freq=0usize))]
    fn insert(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        value: pyo3::PyObject,
        freq: usize,
    ) -> pyo3::PyResult<Option<pyo3::PyObject>> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.entry_with_slot(py, &key)? {
            Entry::Occupied(entry) => Ok(Some(entry.update(value)?)),
            Entry::Absent(entry) => {
                entry.insert(key, value, freq)?;
                Ok(None)
            }
        }
    }

    fn get(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.lookup(py, &key)? {
            Some(val) => Ok(val.clone_ref(py)),
            None => Err(pyo3::PyErr::new::<super::CoreKeyError, _>(key.obj)),
        }
    }

    fn peek(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let lock = self.raw.lock();

        match lock.peek(py, &key)? {
            Some(val) => Ok(val.clone_ref(py)),
            None => Err(pyo3::PyErr::new::<super::CoreKeyError, _>(key.obj)),
        }
    }

    fn update(
        slf: pyo3::PyRef<'_, Self>,
        py: pyo3::Python<'_>,
        iterable: pyo3::PyObject,
    ) -> pyo3::PyResult<()> {
        if slf.as_ptr() == iterable.as_ptr() {
            return Ok(());
        }

        let mut lock = slf.raw.lock();
        lock.extend(py, iterable)
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

                let t1 = slf.raw.lock();
                let t2 = other.raw.lock();
                t1.equal(slf.py(), &t2)
            }
            pyo3::class::basic::CompareOp::Ne => {
                if slf.as_ptr() == other.as_ptr() {
                    return Ok(false);
                }

                let t1 = slf.raw.lock();
                let t2 = other.raw.lock();
                t1.equal(slf.py(), &t2).map(|r| !r)
            }
            _ => Err(pyo3::PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "only '==' or '!=' are supported",
            )),
        }
    }

    fn remove(&self, py: pyo3::Python<'_>, key: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.entry(py, &key)? {
            Entry::Occupied(entry) => {
                let (_, value, _) = entry.remove();
                Ok(value)
            }
            Entry::Absent(_) => Err(pyo3::PyErr::new::<super::CoreKeyError, _>(key.obj)),
        }
    }

    fn popitem(&self) -> pyo3::PyResult<(pyo3::PyObject, pyo3::PyObject)> {
        let mut lock = self.raw.lock();

        match lock.popitem() {
            Some((key, val, _)) => Ok((key.obj, val)),
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

    #[pyo3(signature=(key, default, freq=0usize))]
    fn setdefault(
        &self,
        py: pyo3::Python<'_>,
        key: pyo3::PyObject,
        default: pyo3::PyObject,
        freq: usize,
    ) -> pyo3::PyResult<pyo3::PyObject> {
        let key = PreHashObject::from_pyobject(py, key)?;
        let mut lock = self.raw.lock();

        match lock.entry(py, &key)? {
            Entry::Occupied(entry) => {
                let node = entry.into_value();
                Ok(unsafe { node.as_ref().1.clone_ref(py) })
            }
            Entry::Absent(entry) => {
                entry.insert(key, default.clone_ref(py), freq)?;
                Ok(default)
            }
        }
    }

    fn items(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyResult<pyo3::Py<lfucache_items>> {
        let mut lock = slf.raw.lock();
        let state = lock.observed.get();
        let iter = lock.iter();

        let result = lfucache_items {
            ptr: ObservedIterator::new(slf.as_ptr(), state),
            iter: crate::mutex::Mutex::new(iter),
        };

        pyo3::Py::new(slf.py(), result)
    }

    pub fn least_frequently_used(&self, py: pyo3::Python<'_>, n: usize) -> Option<pyo3::PyObject> {
        let mut lock = self.raw.lock();
        lock.least_frequently_used(n)
            .map(|x| unsafe { x.as_ref().0.obj.clone_ref(py) })
    }

    fn __getnewargs__(&self) -> (usize,) {
        (0,)
    }

    fn __getstate__(&self, py: pyo3::Python<'_>) -> pyo3::PyResult<pyo3::PyObject> {
        let mut lock = self.raw.lock();

        let state = unsafe {
            let list = pyo3::ffi::PyList_New(0);
            if list.is_null() {
                return Err(pyo3::PyErr::fetch(py));
            }

            for ptr in lock.iter() {
                let node = &(*ptr.as_ptr());

                let frequency = pyo3::ffi::PyLong_FromSize_t(node.2);
                if frequency.is_null() {
                    pyo3::ffi::Py_DECREF(list);
                    return Err(pyo3::PyErr::fetch(py));
                }

                let tp = tuple!(
                    py,
                    3,
                    0 => node.0.obj.clone_ref(py).into_ptr(),
                    1 => node.1.clone_ref(py).into_ptr(),
                    2 => frequency,
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

            visit.call(&value.0.obj)?;
            visit.call(&value.1)?;
        }
        Ok(())
    }

    pub fn __clear__(&self) {
        let mut lock = self.raw.lock();
        lock.clear()
    }
}

#[pyo3::pymethods]
impl lfucache_items {
    fn __iter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> {
        slf
    }

    #[allow(unused_mut)]
    fn __next__(mut slf: pyo3::PyRefMut<'_, Self>) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
        let mut iter = slf.iter.lock();

        slf.ptr.proceed(slf.py())?;

        if let Some(x) = iter.next() {
            let (key, val, freq) = unsafe { x.as_ref() };

            let freq = unsafe { pyo3::ffi::PyLong_FromSize_t(*freq) };

            tuple!(
                slf.py(),
                3,
                0 => key.obj.clone_ref(slf.py()).into_ptr(),
                1 => val.clone_ref(slf.py()).into_ptr(),
                2 => freq,
            )
        } else {
            Err(pyo3::PyErr::new::<pyo3::exceptions::PyStopIteration, _>(()))
        }
    }
}
