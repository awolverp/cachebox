use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TryFindMethods;

pub struct NoPolicy {
    table: hashbrown::raw::RawTable<(PreHashObject, pyo3::Py<pyo3::PyAny>, usize)>,
    maxsize: std::num::NonZeroUsize,
    maxmemory: std::num::NonZeroUsize,
    memory: usize,
    pub observed: Observed,
}

pub struct NoPolicyOccupied<'a> {
    instance: &'a mut NoPolicy,
    bucket: hashbrown::raw::Bucket<(PreHashObject, pyo3::Py<pyo3::PyAny>, usize)>,
}

pub struct NoPolicyAbsent<'a> {
    instance: &'a mut NoPolicy,
    insert_slot: Option<hashbrown::raw::InsertSlot>,
}

impl NoPolicy {
    pub fn new(maxsize: usize, mut capacity: usize, maxmemory: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        let maxmemory = non_zero_or!(maxmemory, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            maxsize,
            maxmemory,
            memory: 0,
            observed: Observed::new(),
        })
    }

    pub fn maxsize(&self) -> usize {
        self.maxsize.get()
    }

    pub fn maxmemory(&self) -> usize {
        self.maxmemory.get()
    }

    pub fn memory(&self) -> usize {
        self.memory
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
        self.table.len() == self.maxsize.get() || self.memory >= self.maxmemory.get()
    }

    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    pub fn iter(&self) -> hashbrown::raw::RawIter<(PreHashObject, pyo3::Py<pyo3::PyAny>, usize)> {
        unsafe { self.table.iter() }
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry(
        &'_ mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<NoPolicyOccupied<'_>, NoPolicyAbsent<'_>>> {
        match self.table.try_find(key.hash, |(x, _, _)| x.equal(py, key))? {
            Some(bucket) => {
                Ok(
                    Entry::Occupied(NoPolicyOccupied { instance: self, bucket })
                )
            },
            None => {
                Ok(
                    Entry::Absent(NoPolicyAbsent { instance: self, insert_slot: None })
                )
            }
        }
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry_with_slot(
        &'_ mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<NoPolicyOccupied<'_>, NoPolicyAbsent<'_>>> {
        match self.table.try_find_or_find_insert_slot(
            key.hash,
            |(x, _, _)| x.equal(py, key),
            |(x, _, _)| x.hash,
        )? {
            Ok(bucket) => Ok(
                Entry::Occupied(NoPolicyOccupied { instance: self, bucket })
            ),
            Err(insert_slot) => Ok(
                Entry::Absent(NoPolicyAbsent { instance: self, insert_slot: Some(insert_slot) })
            ),
        }
    }

    #[inline]
    pub fn lookup(
        &self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&pyo3::Py<pyo3::PyAny>>> {
        match self
            .table
            .try_find(key.hash, |(x, _, _)| x.equal(py, key))?
        {
            Some(x) => Ok(Some(unsafe { &x.as_ref().1 })),
            None => Ok(None),
        }
    }

    pub fn equal(&self, py: pyo3::Python<'_>, other: &Self) -> pyo3::PyResult<bool> {
        if self.maxsize != other.maxsize {
            return Ok(false);
        }

        if self.maxmemory != other.maxmemory {
            return Ok(false);
        }

        if self.table.len() != other.table.len() {
            return Ok(false);
        }

        let mut error = None;

        let result = unsafe {
            self.table.iter().all(|bucket| {
                let (key, val, _) = bucket.as_ref();

                match other.table.try_find(key.hash, |(x, _, _)| x.equal(py, key)) {
                    Err(e) => {
                        error = Some(e);
                        true
                    }
                    Ok(Some(bucket)) => {
                        let (_, val2, _) = bucket.as_ref();

                        match crate::common::pyobject_equal(py, val.as_ptr(), val2.as_ptr()) {
                            Ok(result) => result,
                            Err(e) => {
                                error = Some(e);
                                true
                            }
                        }
                    }
                    Ok(None) => false,
                }
            })
        };

        if let Some(error) = error {
            return Err(error);
        }

        Ok(result)
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.memory = 0;
        self.observed.change();
    }

    pub fn shrink_to_fit(&mut self) {
        self.table.shrink_to(self.table.len(), |(x, _, _)| x.hash);
        self.observed.change();
    }

    #[inline]
    pub fn extend(
        &mut self,
        py: pyo3::Python<'_>,
        iterable: pyo3::Py<pyo3::PyAny>,
    ) -> pyo3::PyResult<()> {
        use pyo3::types::{PyAnyMethods, PyDictMethods};

        if unsafe { pyo3::ffi::PyDict_CheckExact(iterable.as_ptr()) == 1 } {
            let dict = unsafe { iterable.cast_bound_unchecked::<pyo3::types::PyDict>(py) };

            for (key, value) in dict.iter() {
                let hk =
                    unsafe { PreHashObject::from_pyobject(py, key.unbind()).unwrap_unchecked() };

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(py, value.unbind())?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(py, hk, value.unbind())?;
                    }
                }
            }
        } else {
            for pair in iterable.bind(py).try_iter()? {
                let (key, value) =
                    pair?.extract::<(pyo3::Py<pyo3::PyAny>, pyo3::Py<pyo3::PyAny>)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match self.entry_with_slot(py, &hk)? {
                    Entry::Occupied(entry) => {
                        entry.update(py, value)?;
                    }
                    Entry::Absent(entry) => {
                        entry.insert(py, hk, value)?;
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
        use pyo3::types::PyDictMethods;

        let (maxsize, iterable, capacity, maxmemory) =
            unsafe { extract_pickle_tuple!(py, state => dict) };

        let mut new = Self::new(maxsize, capacity, maxmemory)?;

        // SAFETY: we checked that the iterable is a dict in extract_pickle_tuple! macro
        let dict = unsafe { iterable.cast_bound_unchecked::<pyo3::types::PyDict>(py) };

        unsafe {
            for (key, value) in dict.iter() {
                let hk = PreHashObject::from_pyobject(py, key.unbind()).unwrap_unchecked();

                match new.entry_with_slot(py, &hk)? {
                    Entry::Absent(entry) => {
                        entry.insert(py, hk, value.unbind())?;
                    }
                    _ => std::hint::unreachable_unchecked(),
                }
            }
        }

        *self = new;
        Ok(())
    }
}

impl<'a> NoPolicyOccupied<'a> {
    #[inline]
    pub fn update(
        self,
        py: pyo3::Python<'_>,
        value: pyo3::Py<pyo3::PyAny>,
    ) -> pyo3::PyResult<pyo3::Py<pyo3::PyAny>> {
        unsafe {
            let item = self.bucket.as_mut();
            let new_size = crate::common::entry_size(py, &item.0, &value)?;

            if new_size > self.instance.maxmemory.get() {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                    "The cache has reached the bound",
                ));
            }

            let next_memory = self
                .instance
                .memory
                .saturating_sub(item.2)
                .saturating_add(new_size);
            if next_memory > self.instance.maxmemory.get() {
                return Err(pyo3::PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                    "The cache has reached the bound",
                ));
            }

            // In update we don't need to change this; because this does not change the memory address ranges
            // self.instance.observed.change();

            let old_value = std::mem::replace(&mut item.1, value);
            item.2 = new_size;
            self.instance.memory = next_memory;
            Ok(old_value)
        }
    }

    #[inline]
    pub fn remove(self) -> (PreHashObject, pyo3::Py<pyo3::PyAny>, usize) {
        let (x, _) = unsafe { self.instance.table.remove(self.bucket) };
        self.instance.memory = self.instance.memory.saturating_sub(x.2);
        self.instance.observed.change();
        x
    }

    pub fn into_value(self) -> &'a mut (PreHashObject, pyo3::Py<pyo3::PyAny>, usize) {
        unsafe { self.bucket.as_mut() }
    }
}

impl NoPolicyAbsent<'_> {
    #[inline]
    pub fn insert(
        self,
        py: pyo3::Python<'_>,
        key: PreHashObject,
        value: pyo3::Py<pyo3::PyAny>,
    ) -> pyo3::PyResult<()> {
        let entry_size = crate::common::entry_size(py, &key, &value)?;

        if entry_size > self.instance.maxmemory.get()
            || self.instance.memory.saturating_add(entry_size) > self.instance.maxmemory.get()
        {
            return Err(pyo3::PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                "The cache has reached the bound",
            ));
        }

        if self.instance.table.len() >= self.instance.maxsize.get() {
            // There's no algorithm for removing a key-value pair, so we raise PyOverflowError.
            return Err(pyo3::PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                "The cache has reached the bound",
            ));
        }

        match self.insert_slot {
            Some(slot) => unsafe {
                self.instance
                    .table
                    .insert_in_slot(key.hash, slot, (key, value, entry_size));
            },
            None => {
                self.instance
                    .table
                    .insert(key.hash, (key, value, entry_size), |(x, _, _)| x.hash);
            }
        }

        self.instance.memory = self.instance.memory.saturating_add(entry_size);
        self.instance.observed.change();
        Ok(())
    }
}
