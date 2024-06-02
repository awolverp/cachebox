use crate::basic::HashablePyObject;
use crate::{create_pyerr, make_eq_func, make_hasher_func};
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;
use std::collections::VecDeque;
use std::time;

pub struct TTLValue(pub PyObject, pub time::Instant);

impl TTLValue {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    #[inline]
    #[must_use]
    pub fn new(val: PyObject, ttl: f32) -> Self {
        TTLValue(
            val,
            time::Instant::now() + time::Duration::from_secs_f32(ttl),
        )
    }

    #[inline]
    pub fn expired(&self) -> bool {
        time::Instant::now() > self.1
    }

    #[inline]
    pub fn remaining(&self) -> f32 {
        (self.1 - time::Instant::now()).as_secs_f32()
    }
}

pub struct RawTTLCache {
    table: RawTable<(HashablePyObject, TTLValue)>,
    order: VecDeque<HashablePyObject>,
    pub ttl: f32,
    pub maxsize: NonZeroUsize,
}

impl RawTTLCache {
    #[inline]
    pub fn new(maxsize: usize, ttl: f32, capacity: usize) -> PyResult<Self> {
        if ttl <= 0.0 {
            return Err(create_pyerr!(
                pyo3::exceptions::PyValueError,
                "ttl value cannot be negative or zero"
            ));
        }

        let capacity = {
            if maxsize != 0 {
                core::cmp::min(maxsize, capacity)
            } else {
                capacity
            }
        };

        let maxsize = unsafe {
            NonZeroUsize::new_unchecked(if maxsize == 0 {
                isize::MAX as usize
            } else {
                maxsize
            })
        };

        let table = {
            if capacity > 0 {
                RawTable::try_with_capacity(capacity)
                    .map_err(|_| create_pyerr!(pyo3::exceptions::PyMemoryError))?
            } else {
                RawTable::new()
            }
        };

        Ok(Self {
            table,
            maxsize,
            ttl,
            order: VecDeque::new(),
        })
    }

    #[inline]
    pub fn popitem(&mut self) -> PyResult<(HashablePyObject, TTLValue)> {
        match self.order.pop_front() {
            Some(x) => {
                #[cfg(debug_assertions)]
                let val = self.table.remove_entry(x.hash, make_eq_func!(x)).unwrap();

                #[cfg(not(debug_assertions))]
                let val = unsafe {
                    self.table
                        .remove_entry(x.hash, make_eq_func!(x))
                        .unwrap_unchecked()
                };

                Ok(val)
            }
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError)),
        }
    }

    #[inline]
    pub fn expire(&mut self) {
        while let Some(key) = self.order.front() {
            match self.table.find(key.hash, make_eq_func!(key)) {
                Some(x) => {
                    let (_, v) = unsafe { x.as_ref() };
                    if !v.expired() {
                        break;
                    }
                }
                None => {
                    #[cfg(debug_assertions)]
                    unreachable!("key not found in order vecdeque: {:?}", key);

                    #[cfg(not(debug_assertions))]
                    unsafe {
                        core::hint::unreachable_unchecked()
                    }
                }
            }

            self.table.remove_entry(key.hash, make_eq_func!(key));
            self.order.pop_front();
        }
    }

    #[inline]
    pub unsafe fn insert_unchecked(&mut self, key: HashablePyObject, value: PyObject) {
        match self
            .table
            .find_or_find_insert_slot(key.hash, make_eq_func!(key), make_hasher_func!())
        {
            Ok(bucket) => {
                let _ = std::mem::replace(
                    unsafe { &mut bucket.as_mut().1 },
                    TTLValue::new(value, self.ttl),
                );
            }
            Err(slot) => unsafe {
                self.table.insert_in_slot(
                    key.hash,
                    slot,
                    (key.clone(), TTLValue::new(value, self.ttl)),
                );
                self.order.push_back(key);
            },
        }
    }

    #[inline]
    pub fn insert(&mut self, key: HashablePyObject, value: PyObject) -> PyResult<()> {
        if self.table.len() >= self.maxsize.get()
            && self.table.find(key.hash, make_eq_func!(key)).is_none()
        {
            #[cfg(debug_assertions)]
            self.popitem().unwrap();

            #[cfg(not(debug_assertions))]
            unsafe {
                self.popitem().unwrap_unchecked()
            };
        }

        unsafe {
            self.insert_unchecked(key, value);
        }
        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &HashablePyObject) -> Option<&TTLValue> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                let (_, val) = unsafe { bucket.as_ref() };

                if val.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(HashablePyObject, TTLValue)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, make_eq_func!(key)) {
            Some(bucket) => {
                let (key, _) = unsafe { bucket.as_ref() };

                #[cfg(debug_assertions)]
                let index = self.order.iter().position(|x| x.eq(key)).unwrap();

                #[cfg(not(debug_assertions))]
                let index = unsafe { self.order.iter().position(|x| x.eq(key)).unwrap_unchecked() };

                self.order.remove(index);

                let (val, _) = unsafe { self.table.remove(bucket) };
                if val.1.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn order_ref(&self) -> &VecDeque<HashablePyObject> {
        &self.order
    }

    #[inline]
    pub fn order_mut(&mut self) -> &mut VecDeque<HashablePyObject> {
        &mut self.order
    }

    #[inline]
    pub fn contains_key(&self, key: &HashablePyObject) -> bool {
        if self.table.is_empty() {
            return false;
        }

        self.table
            .find(key.hash, make_eq_func!(key))
            .filter(|bucket| {
                let (_, v) = unsafe { bucket.as_ref() };
                !v.expired()
            })
            .is_some()
    }

    #[inline]
    pub fn extend_from_dict(&mut self, dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<()> {
        for (key, value) in dict.iter() {
            let hashable = HashablePyObject::try_from_bound(key)?;
            self.insert(hashable, value.unbind())?;
        }

        Ok(())
    }

    #[inline]
    pub fn extend_from_iter(
        &mut self,
        obj: pyo3::Borrowed<'_, '_, PyAny>,
        py: Python<'_>,
    ) -> PyResult<()> {
        for pair in obj.iter()? {
            let (key, value): (Py<PyAny>, Py<PyAny>) = pair?.extract()?;

            let hashable = HashablePyObject::try_from_pyobject(key, py)?;
            self.insert(hashable, value)?;
        }

        Ok(())
    }
}

impl AsRef<RawTable<(HashablePyObject, TTLValue)>> for RawTTLCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(HashablePyObject, TTLValue)> {
        &self.table
    }
}

impl AsMut<RawTable<(HashablePyObject, TTLValue)>> for RawTTLCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(HashablePyObject, TTLValue)> {
        &mut self.table
    }
}
