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
    #[inline]
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

    #[inline]
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

    #[inline]
    pub fn is_full(&self) -> bool {
        self.table.len() == self.maxsize.get()
    }

    #[inline]
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

    #[inline]
    pub fn clear(&mut self) {
        self.table.clear();
        self.heap.clear();
        self.observed.change();
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.table
            .shrink_to(self.table.len(), |x| unsafe { x.as_ref().key.hash });

        self.heap.shrink_to_fit();
        self.observed.change();
    }

    #[inline]
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
}

impl VTTLPolicyOccupied<'_> {
    #[inline]
    pub fn update(
        &mut self,
        value: pyo3::PyObject,
        ttl: Option<f64>,
    ) -> pyo3::PyResult<pyo3::PyObject> {
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

    #[inline]
    pub fn into_value(self) -> NonNull<TimeToLivePair> {
        let item = unsafe { self.bucket.as_mut() };
        *item
    }
}

impl VTTLPolicyAbsent<'_> {
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
