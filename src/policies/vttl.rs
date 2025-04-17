use crate::common::AbsentSituation;
use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TimeToLivePair;
use crate::common::TryFindMethods;
use crate::lazyheap;

use std::ptr::NonNull;

macro_rules! compare_fn {
    () => {
        |a, b| {
            if a.expire_at.is_none() && b.expire_at.is_none() {
                return std::cmp::Ordering::Equal;
            } else if b.expire_at.is_none() {
                return std::cmp::Ordering::Less;
            } else if a.expire_at.is_none() {
                return std::cmp::Ordering::Greater;
            }

            a.expire_at.cmp(&b.expire_at)
        }
    };
}

pub struct VTTLPolicy {
    table: hashbrown::raw::RawTable<NonNull<TimeToLivePair>>,
    heap: lazyheap::LazyHeap<TimeToLivePair>,
    maxsize: std::num::NonZeroUsize,
    pub observed: Observed,
}

pub struct VTTLPolicyOccupied<'a> {
    instance: &'a mut VTTLPolicy,
    bucket: hashbrown::raw::Bucket<NonNull<TimeToLivePair>>,
}

pub struct VTTLPolicyAbsent<'a> {
    instance: &'a mut VTTLPolicy,
    situation: AbsentSituation<NonNull<TimeToLivePair>>,
}

pub type VTTLIterator = lazyheap::Iter<TimeToLivePair>;

impl VTTLPolicy {
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            heap: lazyheap::LazyHeap::new(),
            maxsize,
            observed: Observed::new(),
        })
    }

    pub fn maxsize(&self) -> usize {
        self.maxsize.get()
    }

    #[inline]
    pub fn real_len(&mut self) -> usize {
        self.expire();
        self.table.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.table.len() == self.maxsize.get()
    }

    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    #[inline]
    pub fn expire(&mut self) {
        self.heap.sort_by(compare_fn!());

        let now = std::time::SystemTime::now();

        while let Some(x) = self.heap.front() {
            if unsafe { !x.as_ref().is_expired(now) } {
                break;
            }

            unsafe {
                self.table
                    .remove_entry(x.as_ref().key.hash, |x| {
                        std::ptr::eq(x.as_ptr(), x.as_ptr())
                    })
                    .unwrap();
            }

            self.heap.pop_front(compare_fn!());
            self.observed.change();
        }
    }

    #[inline]
    pub fn popitem(&mut self) -> Option<TimeToLivePair> {
        self.heap.sort_by(compare_fn!());

        let front = self.heap.front()?;

        unsafe {
            self.table
                .remove_entry(front.as_ref().key.hash, |x| {
                    std::ptr::eq(x.as_ptr(), front.as_ptr())
                })
                .unwrap();
        }

        self.observed.change();
        Some(self.heap.pop_front(compare_fn!()).unwrap())
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<VTTLPolicyOccupied, VTTLPolicyAbsent>> {
        match self
            .table
            .try_find(key.hash, |ptr| unsafe { ptr.as_ref().key.equal(py, key) })?
        {
            Some(bucket) => unsafe {
                let pair = bucket.as_ref();

                if !pair.as_ref().is_expired(std::time::SystemTime::now()) {
                    Ok(Entry::Occupied(VTTLPolicyOccupied { instance: self, bucket }))
                } else {
                    Ok(Entry::Absent(VTTLPolicyAbsent { instance: self, situation: AbsentSituation::Expired(bucket) }))
                }
            }
            None => {
                Ok(
                    Entry::Absent(VTTLPolicyAbsent { instance: self, situation: AbsentSituation::None })
                )
            },
        }
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry_with_slot(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<VTTLPolicyOccupied, VTTLPolicyAbsent>> {
        match self
            .table
            .try_find_or_find_insert_slot(
                key.hash,
                |ptr| unsafe { ptr.as_ref().key.equal(py, key) },
                |ptr| unsafe { ptr.as_ref().key.hash },
        )? {
            Ok(bucket) => unsafe {
                let pair = bucket.as_ref();

                if !pair.as_ref().is_expired(std::time::SystemTime::now()) {
                    Ok(Entry::Occupied(VTTLPolicyOccupied { instance: self, bucket }))
                } else {
                    Ok(Entry::Absent(VTTLPolicyAbsent { instance: self, situation: AbsentSituation::Expired(bucket) }))
                }
            }
            Err(slot) => {
                Ok(
                    Entry::Absent(VTTLPolicyAbsent { instance: self, situation: AbsentSituation::Slot(slot) })
                )
            },
        }
    }

    #[inline]
    pub fn lookup(
        &self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&TimeToLivePair>> {
        match self
            .table
            .try_find(key.hash, |ptr| unsafe { ptr.as_ref().key.equal(py, key) })?
            .map(|bucket| unsafe { bucket.as_ref() })
        {
            Some(pair) => unsafe {
                if !pair.as_ref().is_expired(std::time::SystemTime::now()) {
                    Ok(Some(pair.as_ref()))
                } else {
                    Ok(None)
                }
            },
            None => Ok(None),
        }
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.heap.clear();
        self.observed.change();
    }

    pub fn shrink_to_fit(&mut self) {
        self.table
            .shrink_to(self.table.len(), |x| unsafe { x.as_ref().key.hash });

        self.heap.shrink_to_fit();
        self.observed.change();
    }

    pub fn iter(&mut self) -> VTTLIterator {
        self.heap.iter(compare_fn!())
    }

    pub fn equal(&mut self, py: pyo3::Python<'_>, other: &mut Self) -> pyo3::PyResult<bool> {
        if self.maxsize != other.maxsize {
            return Ok(false);
        }

        if self.real_len() != other.real_len() {
            return Ok(false);
        }

        unsafe {
            for node in self.table.iter().map(|x| x.as_ref()) {
                let pair1 = node.as_ref();

                // NOTE: there's no need to check if the pair is expired
                // because we already expired all expired pairs by using real_len method

                match other
                    .table
                    .try_find(pair1.key.hash, |x| pair1.key.equal(py, &x.as_ref().key))?
                {
                    Some(bucket) => {
                        let pair2 = bucket.as_ref().as_ref();

                        if !crate::common::pyobject_equal(
                            py,
                            pair1.value.as_ptr(),
                            pair2.value.as_ptr(),
                        )? {
                            return Ok(false);
                        }
                    }
                    None => return Ok(false),
                }
            }
        }

        Ok(true)
    }

    #[inline]
    pub fn extend(
        &mut self,
        py: pyo3::Python<'_>,
        iterable: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe {
                iterable
                    .downcast_bound::<pyo3::types::PyDict>(py)
                    .unwrap_unchecked()
            };

            for (key, value) in dict.iter() {
                let hk =
                    unsafe { PreHashObject::from_pyobject(py, key.unbind()).unwrap_unchecked() };

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(value.unbind(), ttl)?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(hk, value.unbind(), ttl)?;
                    }
                }
            }
        } else {
            for pair in iterable.bind(py).try_iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(value, ttl)?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(hk, value, ttl)?;
                    }
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_pickle(
        &mut self,
        py: pyo3::Python<'_>,
        state: *mut pyo3::ffi::PyObject,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::PyAnyMethods;

        unsafe {
            tuple!(check state, size=3)?;
            let (maxsize, iterable, capacity) = extract_pickle_tuple!(py, state => list);

            // SAFETY: we check `iterable` type in `extract_pickle_tuple` macro
            if maxsize < (pyo3::ffi::PyObject_Size(iterable.as_ptr()) as usize) {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "iterable object size is greater than maxsize",
                ));
            }

            let mut new = Self::new(maxsize, capacity)?;

            for pair in iterable.bind(py).try_iter()? {
                let (key, value, timestamp) =
                    pair?.extract::<(pyo3::PyObject, pyo3::PyObject, f64)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                let ttl = {
                    if timestamp == 0.0 {
                        None
                    } else {
                        Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(timestamp))
                    }
                };

                match new.entry_with_slot(py, &hk)? {
                    Entry::Absent(entry) => {
                        entry.pickle_insert(hk, value, ttl)?;
                    }
                    _ => std::hint::unreachable_unchecked(),
                }
            }

            new.expire();
            new.shrink_to_fit();

            *self = new;
            Ok(())
        }
    }
}

impl VTTLPolicyOccupied<'_> {
    #[inline]
    pub fn update(self, value: pyo3::PyObject, ttl: Option<f64>) -> pyo3::PyResult<pyo3::PyObject> {
        let item = unsafe { self.bucket.as_mut() };

        unsafe {
            item.as_mut().expire_at =
                ttl.map(|x| std::time::SystemTime::now() + std::time::Duration::from_secs_f64(x));
        }
        self.instance.heap.queue_sort();

        // In update we don't need to change this; because this does not change the memory address ranges
        // self.instance.observed.change();

        Ok(unsafe { std::mem::replace(&mut item.as_mut().value, value) })
    }

    #[inline]
    pub fn remove(self) -> TimeToLivePair {
        let (item, _) = unsafe { self.instance.table.remove(self.bucket) };
        let item = self.instance.heap.remove(item, compare_fn!());

        self.instance.observed.change();
        item
    }

    pub fn into_value(self) -> NonNull<TimeToLivePair> {
        let item = unsafe { self.bucket.as_mut() };
        *item
    }
}

impl VTTLPolicyAbsent<'_> {
    unsafe fn pickle_insert(
        self,
        key: PreHashObject,
        value: pyo3::PyObject,
        expire_at: Option<std::time::SystemTime>,
    ) -> pyo3::PyResult<()> {
        match self.situation {
            AbsentSituation::Expired(_) => {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "pikcle object is suspicious!",
                ))
            }
            AbsentSituation::Slot(slot) => {
                // This means the key is not available and we have insert_slot
                // for inserting it

                // We don't need to check maxsize, we sure `len(iterable) <= maxsize` in loading pickle

                let hash = key.hash;
                let node = self
                    .instance
                    .heap
                    .push(TimeToLivePair::new(key, value, expire_at));

                unsafe {
                    self.instance.table.insert_in_slot(hash, slot, node);
                }
            }
            AbsentSituation::None => unsafe { std::hint::unreachable_unchecked() },
        }

        Ok(())
    }

    #[inline]
    pub fn insert(
        self,
        key: PreHashObject,
        value: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<()> {
        let expire_at =
            ttl.map(|x| std::time::SystemTime::now() + std::time::Duration::from_secs_f64(x));

        match self.situation {
            AbsentSituation::Expired(bucket) => {
                // This means the key is available but expired
                // So we have to update the values of the old key
                // and queue the heap's sort
                let item = unsafe { bucket.as_mut() };

                unsafe {
                    item.as_mut().expire_at = ttl.map(|x| {
                        std::time::SystemTime::now() + std::time::Duration::from_secs_f64(x)
                    });
                    item.as_mut().value = value;
                }

                self.instance.heap.queue_sort();

                // Like VTTLPolicyOccupied::update, Here we don't need to change this
                // self.instance.observed.change();
            }
            AbsentSituation::Slot(slot) => {
                self.instance.expire(); // Remove expired pairs to make room for the new pair

                if self.instance.table.len() >= self.instance.maxsize.get() {
                    self.instance.popitem();
                }

                let hash = key.hash;
                let node = self
                    .instance
                    .heap
                    .push(TimeToLivePair::new(key, value, expire_at));

                unsafe {
                    self.instance.table.insert_in_slot(hash, slot, node);
                }

                self.instance.observed.change();
            }
            AbsentSituation::None => {
                self.instance.expire(); // Remove expired pairs to make room for the new pair

                if self.instance.table.len() >= self.instance.maxsize.get() {
                    self.instance.popitem();
                }

                let hash = key.hash;
                let node = self
                    .instance
                    .heap
                    .push(TimeToLivePair::new(key, value, expire_at));

                self.instance
                    .table
                    .insert(hash, node, |x| unsafe { x.as_ref().key.hash });

                self.instance.observed.change();
            }
        }

        Ok(())
    }
}

unsafe impl Send for VTTLPolicy {}
