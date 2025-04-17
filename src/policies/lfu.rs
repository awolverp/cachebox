use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TryFindMethods;
use crate::lazyheap;
use std::ptr::NonNull;

type TupleValue = (PreHashObject, pyo3::PyObject, usize);

pub struct LFUPolicy {
    table: hashbrown::raw::RawTable<NonNull<TupleValue>>,
    heap: lazyheap::LazyHeap<TupleValue>,
    maxsize: std::num::NonZeroUsize,
    pub observed: Observed,
}

pub struct LFUPolicyOccupied<'a> {
    instance: &'a mut LFUPolicy,
    bucket: hashbrown::raw::Bucket<NonNull<TupleValue>>,
}

pub struct LFUPolicyAbsent<'a> {
    instance: &'a mut LFUPolicy,
    insert_slot: Option<hashbrown::raw::InsertSlot>,
}

pub type LFUIterator = lazyheap::Iter<(PreHashObject, pyo3::Py<pyo3::PyAny>, usize)>;

impl LFUPolicy {
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
    pub fn len(&self) -> usize {
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
    pub fn popitem(&mut self) -> Option<TupleValue> {
        self.heap.sort_by(|a, b| a.2.cmp(&b.2));
        let front = self.heap.front()?;

        unsafe {
            self.table
                .remove_entry(front.as_ref().0.hash, |x| {
                    std::ptr::eq(x.as_ptr(), front.as_ptr())
                })
                .unwrap();
        }

        self.observed.change();
        Some(self.heap.pop_front(|a, b| a.2.cmp(&b.2)).unwrap())
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<LFUPolicyOccupied, LFUPolicyAbsent>> {
        match self
            .table
            .try_find(key.hash, |ptr| unsafe { ptr.as_ref().0.equal(py, key) })?
        {
            Some(bucket) => {
                Ok(
                    Entry::Occupied(LFUPolicyOccupied { instance: self, bucket })
                )
            },
            None => {
                Ok(
                    Entry::Absent(LFUPolicyAbsent { instance: self, insert_slot: None })
                )
            }
        }
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry_with_slot(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<LFUPolicyOccupied, LFUPolicyAbsent>> {
        match self.table.try_find_or_find_insert_slot(
                key.hash,
                |ptr| unsafe { ptr.as_ref().0.equal(py, key) },
                |ptr| unsafe { ptr.as_ref().0.hash },
        )? {
            Ok(bucket) => {
                Ok(
                    Entry::Occupied(LFUPolicyOccupied { instance: self, bucket })
                )
            },
            Err(slot) => {
                Ok(
                    Entry::Absent(LFUPolicyAbsent { instance: self, insert_slot: Some(slot) })
                )
            }
        }
    }

    #[inline]
    pub fn lookup(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&pyo3::PyObject>> {
        match self.entry(py, key)? {
            Entry::Occupied(x) => unsafe {
                x.bucket.as_mut().as_mut().2 += 1;
                x.instance.heap.queue_sort();

                Ok(Some(&x.bucket.as_ref().as_ref().1))
            },
            Entry::Absent(_) => Ok(None),
        }
    }

    pub fn peek(
        &self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&pyo3::PyObject>> {
        let result = self
            .table
            .try_find(key.hash, |x| unsafe { x.as_ref().0.equal(py, key) })?
            .map(|x| unsafe { &x.as_ref().as_ref().1 });

        Ok(result)
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.heap.clear();
        self.observed.change();
    }

    pub fn shrink_to_fit(&mut self) {
        self.table
            .shrink_to(self.table.len(), |x| unsafe { x.as_ref().0.hash });

        self.heap.shrink_to_fit();
        self.observed.change();
    }

    pub fn equal(&self, py: pyo3::Python<'_>, other: &Self) -> pyo3::PyResult<bool> {
        if self.maxsize != other.maxsize {
            return Ok(false);
        }

        if self.table.len() != other.table.len() {
            return Ok(false);
        }

        unsafe {
            for node in self.table.iter().map(|x| x.as_ref()) {
                let (key1, value1, _) = node.as_ref();

                match other
                    .table
                    .try_find(key1.hash, |x| key1.equal(py, &x.as_ref().0))?
                {
                    Some(bucket) => {
                        let (_, value2, _) = bucket.as_ref().as_ref();

                        if !crate::common::pyobject_equal(py, value1.as_ptr(), value2.as_ptr())? {
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
    pub fn extend(&mut self, py: pyo3::Python<'_>, iterable: pyo3::PyObject) -> pyo3::PyResult<()> {
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
                        entry.update(value.unbind())?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(hk, value.unbind(), 0)?;
                    }
                }
            }
        } else {
            for pair in iterable.bind(py).try_iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(value)?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(hk, value, 0)?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn iter(&mut self) -> LFUIterator {
        self.heap.iter(|a, b| a.2.cmp(&b.2))
    }

    pub fn least_frequently_used(&mut self, n: usize) -> Option<NonNull<TupleValue>> {
        self.heap.sort_by(|a, b| a.2.cmp(&b.2));
        let node = self.heap.get(n)?;

        Some(*node)
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
                let (key, value, freq) =
                    pair?.extract::<(pyo3::PyObject, pyo3::PyObject, usize)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match new.entry_with_slot(py, &hk)? {
                    Entry::Absent(entry) => {
                        entry.insert(hk, value, freq)?;
                    }
                    _ => std::hint::unreachable_unchecked(),
                }
            }

            new.heap.sort_by(|a, b| a.2.cmp(&b.2));

            *self = new;
            Ok(())
        }
    }
}

impl LFUPolicyOccupied<'_> {
    #[inline]
    pub fn update(self, value: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        let item = unsafe { self.bucket.as_mut() };
        unsafe {
            item.as_mut().2 += 1;
        }

        self.instance.heap.queue_sort();

        // In update we don't need to change this; because this does not change the memory address ranges
        // self.instance.observed.change();

        Ok(unsafe { std::mem::replace(&mut item.as_mut().1, value) })
    }

    #[inline]
    pub fn remove(self) -> TupleValue {
        let (item, _) = unsafe { self.instance.table.remove(self.bucket) };
        let item = self.instance.heap.remove(item, |a, b| a.2.cmp(&b.2));

        self.instance.observed.change();
        item
    }

    pub fn into_value(self) -> NonNull<TupleValue> {
        let item = unsafe { self.bucket.as_mut() };
        *item
    }
}

impl LFUPolicyAbsent<'_> {
    #[inline]
    pub fn insert(
        self,
        key: PreHashObject,
        value: pyo3::PyObject,
        freq: usize,
    ) -> pyo3::PyResult<()> {
        if self.instance.table.len() >= self.instance.maxsize.get() {
            self.instance.popitem();
        }

        let hash = key.hash;
        let node = self.instance.heap.push((key, value, freq));

        match self.insert_slot {
            Some(slot) => unsafe {
                self.instance.table.insert_in_slot(hash, slot, node);
            },
            None => {
                self.instance
                    .table
                    .insert(hash, node, |x| unsafe { x.as_ref().0.hash });
            }
        }

        self.instance.observed.change();
        Ok(())
    }
}

unsafe impl Send for LFUPolicy {}
