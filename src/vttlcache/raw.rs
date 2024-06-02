use crate::basic::HashablePyObject;
use crate::create_pyerr;
use core::num::NonZeroUsize;
use hashbrown::raw::RawTable;
use pyo3::prelude::*;
use std::time;

macro_rules! make_eq_func {
    ($key:expr) => {
        |(x, _)| x.key() == $key.key()
    };
}

macro_rules! sort_keys {
    ($order:expr) => {
        $order.sort_by(|a, b| {
            let ap = a.expiration();
            let bp = b.expiration();

            if ap.is_none() && bp.is_none() {
                return std::cmp::Ordering::Equal;
            }
            if bp.is_none() {
                return std::cmp::Ordering::Greater;
            }
            if ap.is_none() {
                return std::cmp::Ordering::Less;
            }
            bp.cmp(&ap)
        });
    };
}

#[derive(Clone, Debug)]
pub enum VTTLKey {
    NoExpire(HashablePyObject),
    Expire(HashablePyObject, time::Instant),
}

impl VTTLKey {
    pub const SIZE: usize = core::mem::size_of::<VTTLKey>();

    #[must_use]
    #[inline]
    pub fn new(val: HashablePyObject, ttl: Option<f32>) -> Self {
        match ttl {
            Some(x) => Self::Expire(val, time::Instant::now() + time::Duration::from_secs_f32(x)),
            None => Self::NoExpire(val),
        }
    }

    #[inline]
    pub fn expired(&self) -> bool {
        match *self {
            Self::Expire(_, ref ttl) => time::Instant::now() > *ttl,
            Self::NoExpire(_) => false,
        }
    }

    #[inline]
    pub fn key(&self) -> &HashablePyObject {
        match self {
            Self::NoExpire(val) => val,
            Self::Expire(val, _) => val,
        }
    }

    #[inline]
    pub fn into_key(self) -> HashablePyObject {
        match self {
            Self::NoExpire(val) => val,
            Self::Expire(val, _) => val,
        }
    }

    #[inline]
    pub fn expiration(&self) -> Option<&time::Instant> {
        match self {
            Self::NoExpire(_) => None,
            Self::Expire(_, instant) => Some(instant),
        }
    }

    #[inline]
    pub fn remaining(&self) -> Option<f32> {
        self.expiration()
            .map(|instant| (*instant - time::Instant::now()).as_secs_f32())
    }
}

pub struct RawVTTLCache {
    table: RawTable<(VTTLKey, PyObject)>,
    order: Vec<VTTLKey>,
    pub maxsize: NonZeroUsize,
}

impl RawVTTLCache {
    #[inline]
    pub fn new(maxsize: usize, capacity: usize) -> PyResult<Self> {
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
            order: Vec::new(),
        })
    }

    #[inline]
    pub fn popitem(&mut self) -> PyResult<(VTTLKey, PyObject)> {
        match self.order.pop() {
            Some(key) => {
                #[cfg(debug_assertions)]
                let val = self
                    .table
                    .remove_entry(key.key().hash, make_eq_func!(key))
                    .unwrap();

                #[cfg(not(debug_assertions))]
                let val = unsafe {
                    self.table
                        .remove_entry(key.key().hash, make_eq_func!(key))
                        .unwrap_unchecked()
                };

                Ok(val)
            }
            None => Err(create_pyerr!(pyo3::exceptions::PyKeyError)),
        }
    }

    #[inline]
    pub fn expire(&mut self) {
        while let Some(key) = self.order.last() {
            if !key.expired() {
                break;
            }

            self.table.remove_entry(key.key().hash, make_eq_func!(key));
            self.order.pop();
        }
    }

    #[inline]
    pub unsafe fn insert_unsorted(
        &mut self,
        key: HashablePyObject,
        value: PyObject,
        ttl: Option<f32>,
    ) {
        if self.table.len() >= self.maxsize.get()
            && self
                .table
                .find(key.hash, |(x, _)| *x.key() == key)
                .is_none()
        {
            #[cfg(debug_assertions)]
            self.popitem().unwrap();

            #[cfg(not(debug_assertions))]
            unsafe {
                self.popitem().unwrap_unchecked()
            };
        }

        match self.table.find_or_find_insert_slot(
            key.hash,
            |(x, _)| *x.key() == key,
            |(x, _)| x.key().hash,
        ) {
            Ok(bucket) => {
                let _ =
                    std::mem::replace(unsafe { &mut bucket.as_mut().0 }, VTTLKey::new(key, ttl));
                let _ = std::mem::replace(unsafe { &mut bucket.as_mut().1 }, value);
            }
            Err(slot) => unsafe {
                let k = VTTLKey::new(key.clone(), ttl);
                self.table
                    .insert_in_slot(key.hash, slot, (k.clone(), value));
                self.order.push(k);
            },
        }
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: HashablePyObject,
        value: PyObject,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        unsafe {
            self.insert_unsorted(key, value, ttl);
        }

        if self.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(self.order);
        }

        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &HashablePyObject) -> Option<(f32, &PyObject)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, |(x, _)| x.key() == key) {
            Some(bucket) => {
                let (x, val) = unsafe { bucket.as_ref() };

                if x.expired() {
                    None
                } else {
                    Some((x.remaining().unwrap_or(0.0), val))
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn remove(&mut self, key: &HashablePyObject) -> Option<(VTTLKey, PyObject)> {
        if self.table.is_empty() {
            return None;
        }

        match self.table.find(key.hash, |(x, _)| x.key() == key) {
            Some(bucket) => {
                // override key variable
                let (key, _) = unsafe { bucket.as_ref() };

                #[cfg(debug_assertions)]
                let index = self
                    .order
                    .iter()
                    .position(|x| x.key() == key.key())
                    .unwrap();

                #[cfg(not(debug_assertions))]
                let index = unsafe {
                    self.order
                        .iter()
                        .position(|x| x.key() == key.key())
                        .unwrap_unchecked()
                };

                self.order.swap_remove(index);

                let (val, _) = unsafe { self.table.remove(bucket) };
                if val.0.expired() {
                    None
                } else {
                    Some(val)
                }
            }
            None => None,
        }
    }

    #[inline]
    pub fn order_ref(&self) -> &Vec<VTTLKey> {
        &self.order
    }

    #[inline]
    pub fn order_mut(&mut self) -> &mut Vec<VTTLKey> {
        &mut self.order
    }

    #[inline]
    pub fn contains_key(&self, key: &HashablePyObject) -> bool {
        if self.table.is_empty() {
            return false;
        }

        self.table
            .find(key.hash, |(x, _)| x.key() == key)
            .filter(|bucket| {
                let (k, _) = unsafe { bucket.as_ref() };
                !k.expired()
            })
            .is_some()
    }

    #[inline]
    pub fn extend_from_dict(
        &mut self,
        dict: &Bound<'_, pyo3::types::PyDict>,
        ttl: Option<f32>,
    ) -> PyResult<()> {
        for (key, value) in dict.iter() {
            let hashable = HashablePyObject::try_from_bound(key)?;
            unsafe {
                self.insert_unsorted(hashable, value.unbind(), ttl);
            }
        }

        if self.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(self.order);
        }

        Ok(())
    }

    #[inline]
    pub fn extend_from_iter(
        &mut self,
        obj: pyo3::Borrowed<'_, '_, PyAny>,
        ttl: Option<f32>,
        py: Python<'_>,
    ) -> PyResult<()> {
        for pair in obj.iter()? {
            let (key, value): (Py<PyAny>, Py<PyAny>) = pair?.extract()?;
            let hashable = HashablePyObject::try_from_pyobject(key, py)?;
            unsafe {
                self.insert_unsorted(hashable, value, ttl);
            }
        }

        if self.order.len() > 1 {
            // Sort from less to greater
            sort_keys!(self.order);
        }

        Ok(())
    }
}

impl AsRef<RawTable<(VTTLKey, PyObject)>> for RawVTTLCache {
    #[inline]
    fn as_ref(&self) -> &RawTable<(VTTLKey, PyObject)> {
        &self.table
    }
}

impl AsMut<RawTable<(VTTLKey, PyObject)>> for RawVTTLCache {
    #[inline]
    fn as_mut(&mut self) -> &mut RawTable<(VTTLKey, PyObject)> {
        &mut self.table
    }
}
