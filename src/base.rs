use pyo3::prelude::*;
use std::time;

pub const ISIZE_MEMORY_SIZE: usize = std::mem::size_of::<isize>();
pub const INSTANT_MEMORY_SIZE: usize = std::mem::size_of::<time::Instant>();

pub trait CacheImplemention {
    type Pair;

    fn new(maxsize: usize, capacity: usize) -> Self;
    // fn getitem(&self, hash: &isize) -> Option<&Self::Pair>;
    // Cannot implement getitem, because cannot return referecing value when using RwLock.
    fn cache_popitem(&mut self) -> Option<Self::Pair>;
    fn cache_setitem(&mut self, hash: isize, key: Py<PyAny>, value: Py<PyAny>) -> PyResult<()>;
    fn cache_remove(&mut self, hash: &isize) -> Option<Self::Pair>;
    fn cache_len(&self) -> usize;
    fn cache_contains(&self, hash: &isize) -> bool;
    fn cache_clear(&mut self, reuse: bool);
    fn cache_sizeof(&self) -> usize;
    fn cache_keys(&self) -> Vec<Py<PyAny>>;
    fn cache_values(&self) -> Vec<Py<PyAny>>;
    fn cache_items(&self) -> Vec<(Py<PyAny>, Py<PyAny>)>;
    fn cache_equal(&self, other: &Self) -> bool;
    fn cache_update_from_pydict(&mut self, other: &pyo3::types::PyDict) -> PyResult<()>;
    fn cache_update_from_pyobject(&mut self, other: &pyo3::types::PyIterator) -> PyResult<()>;
}

#[derive(Clone)]
pub struct KeyValuePair(pub Py<PyAny>, pub Py<PyAny>);

#[derive(Clone, Debug)]
pub struct TTLKeyValuePair {
    pub key: Py<PyAny>,
    pub value: Py<PyAny>,
    pub expire: Option<time::Instant>,
}

impl TTLKeyValuePair {
    pub fn is_expired(&self) -> bool {
        match self.expire {
            Some(v) => time::Instant::now() >= v,
            None => false,
        }
    }
}

#[pyclass(subclass)]
pub struct BaseCacheImpl {}

#[pymethods]
impl BaseCacheImpl {
    #[new]
    #[pyo3(signature=(maxsize, *, capacity=0))]
    pub fn __new__(maxsize: usize, capacity: usize) -> PyResult<Self> {
        let _ = maxsize;
        let _ = capacity;
        Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "This type is not implemented and baseclass of other classes, use other implements.",
        ))
    }
}

#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! implement_default_functions {
        ($class:ty) => {
            #[pymethods]
            impl $class {
                #[new]
                #[pyo3(signature=(maxsize, *, capacity=0))]
                pub fn __new__(maxsize: usize, capacity: usize) -> (Self, base::BaseCacheImpl) {
                    (<$class>::new(maxsize, capacity), base::BaseCacheImpl {})
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

                pub fn __len__(&self) -> PyResult<usize> {
                    Ok(self.cache_len())
                }

                pub fn __repr__(&self) -> PyResult<String> {
                    let read = self.inner.read().expect("RwLock is poisoned (read)");
                    Ok(format!(
                        "<cachebox._cachebox.{} len={} maxsize={} capacity={}>",
                        stringify!($class),
                        read.len(),
                        self.maxsize,
                        read.capacity()
                    ))
                }

                pub fn __sizeof__(&self) -> PyResult<usize> {
                    Ok(self.cache_sizeof())
                }

                pub fn __richcmp__(
                    &self,
                    other: &Self,
                    op: pyo3::class::basic::CompareOp,
                ) -> PyResult<bool> {
                    match op {
                        pyo3::class::basic::CompareOp::Eq => Ok(self.cache_equal(other)),
                        pyo3::class::basic::CompareOp::Ne => Ok(!self.cache_equal(other)),
                        _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                            "only == and != operations are supported",
                        )),
                    }
                }

                pub fn insert(
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

                pub fn keys(&self) -> PyResult<Vec<Py<PyAny>>> {
                    Ok(self.cache_keys())
                }

                pub fn values(&self) -> PyResult<Vec<Py<PyAny>>> {
                    Ok(self.cache_values())
                }

                pub fn items(&self) -> PyResult<Vec<(Py<PyAny>, Py<PyAny>)>> {
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
                        Some(v) => Ok(Some(v.1)),
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
                        return Ok(Some(v.1.clone()));
                    } else {
                        drop(read);

                        let defaultvalue: Py<PyAny>;
                        if let Some(v) = default {
                            defaultvalue = v;
                        } else {
                            defaultvalue = py.None();
                        }

                        match self.cache_setitem(hash, key, defaultvalue.clone()) {
                            Ok(_) => {
                                return Ok(Some(defaultvalue));
                            }
                            Err(err) => {
                                drop(defaultvalue);
                                return Err(err);
                            }
                        }
                    }
                }

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
            }
        };
    }
}
