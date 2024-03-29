use parking_lot::RwLock;
use pyo3::prelude::*;

use crate::classes::base;
use crate::internal;

#[pyclass(extends=base::BaseCacheImpl, subclass, module = "cachebox._cachebox")]
pub struct VTTLCache {
    pub inner: RwLock<internal::VTTLCache<isize, base::KeyValuePair>>,
}

#[pymethods]
impl VTTLCache {
    #[new]
    #[pyo3(signature=(maxsize, iterable=None, ttl=None, *, capacity=0))]
    fn __new__(
        py: Python<'_>,
        maxsize: usize,
        iterable: Option<Py<PyAny>>,
        ttl: Option<f32>,
        capacity: usize,
    ) -> PyResult<(Self, base::BaseCacheImpl)> {
        let (mut slf, base) = (
            VTTLCache {
                inner: RwLock::new(internal::VTTLCache::new(maxsize, capacity)),
            },
            base::BaseCacheImpl {},
        );

        if let Some(x) = iterable {
            slf.update(py, x, ttl)?;
        }

        Ok((slf, base))
    }

    #[getter]
    fn maxsize(&self) -> usize {
        self.inner.read().maxsize
    }

    fn getmaxsize(&self) -> usize {
        self.inner.read().maxsize
    }

    fn __len__(&mut self) -> usize {
        let mut write = self.inner.write();
        write.expire();
        write.len()
    }

    fn __sizeof__(&self) -> usize {
        let read = self.inner.read();
        let cap = read.capacity();

        (cap * base::ISIZE_MEMORY_SIZE)
            + (cap * base::PYOBJECT_MEMORY_SIZE)
            + (read.order_capacity() * base::ISIZE_MEMORY_SIZE)
            + base::ISIZE_MEMORY_SIZE
    }

    fn __bool__(&mut self) -> bool {
        let mut write = self.inner.write();
        write.expire();
        !write.is_empty()
    }

    fn __setitem__(&mut self, py: Python<'_>, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()> {
        let hash = pyany_to_hash!(key, py)?;
        let mut write = self.inner.write();
        write.expire();
        write.insert(hash, base::KeyValuePair(key, value), None)
    }

    fn insert(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        value: Py<PyAny>,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        let hash = pyany_to_hash!(key, py)?;
        let mut write = self.inner.write();
        write.expire();
        write.insert(hash, base::KeyValuePair(key, value), ttl)
    }

    fn __getitem__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
        let hash = pyany_to_hash!(key, py)?;

        match self.inner.read().get(&hash) {
            Some(val) => Ok(val.value.1.clone()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    fn get(
        &self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let hash = pyany_to_hash!(key, py)?;

        match self.inner.read().get(&hash) {
            Some(val) => Ok(val.value.1.clone()),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    fn __delitem__(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        let hash = pyany_to_hash!(key, py)?;

        match self.inner.write().remove(&hash) {
            Some(_) => Ok(()),
            None => Err(pyo3::exceptions::PyKeyError::new_err(key)),
        }
    }

    fn delete(&mut self, py: Python<'_>, key: Py<PyAny>) -> PyResult<()> {
        self.__delitem__(py, key)
    }

    fn __contains__(&self, py: Python<'_>, key: Py<PyAny>) -> PyResult<bool> {
        let hash = pyany_to_hash!(key, py)?;
        Ok(self.inner.read().contains_key(&hash))
    }

    fn __eq__(&self, other: &Self) -> bool {
        let map1 = self.inner.read();
        let map2 = other.inner.read();
        map1.eq(&map2)
    }

    fn __ne__(&self, other: &Self) -> bool {
        let map1 = self.inner.read();
        let map2 = other.inner.read();
        map1.ne(&map2)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<base::VecOneValueIterator>> {
        let mut write = slf.inner.write();
        write.expire();

        let view: Vec<Py<PyAny>> = write
            .sorted_keys()
            .map(|x| write.get(x).unwrap().value.0.clone())
            .collect();

        let iter = base::VecOneValueIterator {
            view: view.into_iter(),
        };

        Py::new(slf.py(), iter)
    }

    fn keys(slf: PyRef<'_, Self>) -> PyResult<Py<base::VecOneValueIterator>> {
        let mut write = slf.inner.write();
        write.expire();

        let view: Vec<Py<PyAny>> = write
            .sorted_keys()
            .map(|x| write.get(x).unwrap().value.0.clone())
            .collect();

        let iter = base::VecOneValueIterator {
            view: view.into_iter(),
        };

        Py::new(slf.py(), iter)
    }

    fn values(slf: PyRef<'_, Self>) -> PyResult<Py<base::VecOneValueIterator>> {
        let mut write = slf.inner.write();
        write.expire();

        let view: Vec<Py<PyAny>> = write
            .sorted_keys()
            .map(|x| write.get(x).unwrap().value.1.clone())
            .collect();

        let iter = base::VecOneValueIterator {
            view: view.into_iter(),
        };

        Py::new(slf.py(), iter)
    }

    fn items(slf: PyRef<'_, Self>) -> PyResult<Py<base::VecItemsIterator>> {
        let mut write = slf.inner.write();
        write.expire();

        let view: Vec<(Py<PyAny>, Py<PyAny>)> = write
            .sorted_keys()
            .map(|x| {
                let val = write.get(x).unwrap();
                (val.value.0.clone(), val.value.1.clone())
            })
            .collect();

        let iter = base::VecItemsIterator {
            view: view.into_iter(),
        };

        Py::new(slf.py(), iter)
    }

    fn __repr__(slf: &PyCell<Self>) -> PyResult<String> {
        let class_name: &str = slf.get_type().name()?;
        let mut borrowed = slf.borrow_mut();
        Ok(format!(
            "{}({} / {}, capacity={})",
            class_name,
            borrowed.__len__(),
            borrowed.maxsize(),
            borrowed.capacity()
        ))
    }

    fn capacity(&self) -> usize {
        self.inner.read().capacity()
    }

    #[pyo3(signature=(*, reuse=false))]
    fn clear(&mut self, reuse: bool) {
        self.inner.write().clear(reuse);
    }

    #[pyo3(signature=(key, default=None))]
    fn pop(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let hash = pyany_to_hash!(key, py)?;

        match self.inner.write().remove(&hash) {
            Some(x) => Ok(x.value.1),
            None => Ok(default.unwrap_or_else(|| py.None())),
        }
    }

    #[pyo3(signature=(key, default=None, ttl=None))]
    fn setdefault(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
        ttl: Option<f32>,
    ) -> PyResult<Py<PyAny>> {
        let hash = pyany_to_hash!(key, py)?;
        let default_val = default.unwrap_or_else(|| py.None());

        match self
            .inner
            .write()
            .setdefault(hash, base::KeyValuePair(key, default_val), ttl)
        {
            Ok(x) => Ok(x.1),
            Err(s) => Err(s),
        }
    }

    fn popitem(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
        match self.inner.write().popitem() {
            Some(val) => Ok((val.value.0, val.value.1)),
            None => Err(pyo3::exceptions::PyKeyError::new_err(())),
        }
    }

    fn drain(&mut self, n: usize) -> usize {
        self.inner.write().drain(n)
    }

    #[pyo3(signature=(iterable, ttl=None))]
    fn update(&mut self, py: Python<'_>, iterable: Py<PyAny>, ttl: Option<f32>) -> PyResult<()> {
        let obj = iterable.as_ref(py);

        if obj.is_instance_of::<pyo3::types::PyDict>() {
            let dict = obj.downcast::<pyo3::types::PyDict>()?;

            let mut write = self.inner.write();
            write.expire();

            write.update(
                dict.iter().map(|(key, val)| {
                    Ok::<(isize, base::KeyValuePair), PyErr>((
                        unsafe { key.hash().unwrap_unchecked() },
                        base::KeyValuePair(key.into(), val.into()),
                    ))
                }),
                ttl,
            )?;
        } else {
            let iter = obj.iter()?;

            let mut write = self.inner.write();
            write.expire();

            write.update(
                iter.map(|key| {
                    let items: (&PyAny, &PyAny) = key?.extract()?;
                    let hash = items.0.hash()?;

                    Ok::<(isize, base::KeyValuePair), PyErr>((
                        hash,
                        base::KeyValuePair(items.0.into(), items.1.into()),
                    ))
                }),
                ttl,
            )?;
        }

        Ok(())
    }

    fn shrink_to_fit(&mut self) {
        self.inner.write().shrink_to_fit();
    }

    fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        for value in self.inner.read().values() {
            visit.call(&value.value.0)?;
            visit.call(&value.value.1)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.inner.write().clear(false);
    }

    #[pyo3(signature=(key, default=None))]
    fn get_with_expire(
        &self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<(Py<PyAny>, f32)> {
        let hash = pyany_to_hash!(key, py)?;

        match self.inner.read().get(&hash) {
            Some(val) => {
                let ex = match val.expiration {
                    Some(ex) => ex - std::time::Instant::now(),
                    None => std::time::Duration::new(0, 0),
                };
                Ok((val.value.1.clone(), ex.as_secs_f32()))
            }
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    #[pyo3(signature=(key, default=None))]
    fn pop_with_expire(
        &mut self,
        py: Python<'_>,
        key: Py<PyAny>,
        default: Option<Py<PyAny>>,
    ) -> PyResult<(Py<PyAny>, f32)> {
        let hash = pyany_to_hash!(key, py)?;

        match self.inner.write().remove(&hash) {
            Some(val) => {
                let ex = match val.expiration {
                    Some(ex) => ex - std::time::Instant::now(),
                    None => std::time::Duration::new(0, 0),
                };
                Ok((val.value.1.clone(), ex.as_secs_f32()))
            }
            None => Ok((default.unwrap_or_else(|| py.None()), 0.0)),
        }
    }

    fn popitem_with_expire(&mut self) -> PyResult<(Py<PyAny>, Py<PyAny>, f32)> {
        match self.inner.write().popitem() {
            Some(val) => {
                let ex = match val.expiration {
                    Some(ex) => ex - std::time::Instant::now(),
                    None => std::time::Duration::new(0, 0),
                };
                Ok((val.value.0, val.value.1, ex.as_secs_f32()))
            }
            None => Err(pyo3::exceptions::PyKeyError::new_err(())),
        }
    }
}
