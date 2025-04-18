use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TryFindMethods;

pub struct RandomPolicy {
    table: hashbrown::raw::RawTable<(PreHashObject, pyo3::PyObject)>,
    maxsize: std::num::NonZeroUsize,
    pub observed: Observed,
}

pub struct RandomPolicyOccupied<'a> {
    instance: &'a mut RandomPolicy,
    bucket: hashbrown::raw::Bucket<(PreHashObject, pyo3::PyObject)>,
}

pub struct RandomPolicyAbsent<'a> {
    instance: &'a mut RandomPolicy,
    insert_slot: Option<hashbrown::raw::InsertSlot>,
}

impl RandomPolicy {
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
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

    pub fn iter(&self) -> hashbrown::raw::RawIter<(PreHashObject, pyo3::PyObject)> {
        unsafe { self.table.iter() }
    }

    #[inline]
    pub fn popitem(&mut self) -> pyo3::PyResult<Option<(PreHashObject, pyo3::PyObject)>> {
        if self.table.is_empty() {
            Ok(None)
        } else {
            let nth = fastrand::usize(0..self.table.len());

            let bucket = unsafe { self.table.iter().nth(nth).unwrap_unchecked() };
            let (x, _) = unsafe { self.table.remove(bucket) };

            self.observed.change();
            Ok(Some(x))
        }
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<RandomPolicyOccupied, RandomPolicyAbsent>> {
        match self.table.try_find(key.hash, |(x, _)| x.equal(py, key))? {
            Some(bucket) => {
                Ok(
                    Entry::Occupied(RandomPolicyOccupied { instance: self, bucket })
                )
            },
            None => {
                Ok(
                    Entry::Absent(RandomPolicyAbsent { instance: self, insert_slot: None })
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
    ) -> pyo3::PyResult<Entry<RandomPolicyOccupied, RandomPolicyAbsent>> {
        match self.table.try_find_or_find_insert_slot(
            key.hash,
            |(x, _)| x.equal(py, key),
            |(x, _)| x.hash,
        )? {
            Ok(bucket) => Ok(
                Entry::Occupied(RandomPolicyOccupied { instance: self, bucket })
            ),
            Err(insert_slot) => Ok(
                Entry::Absent(RandomPolicyAbsent { instance: self, insert_slot: Some(insert_slot) })
            ),
        }
    }

    #[inline]
    pub fn lookup(
        &self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Option<&pyo3::PyObject>> {
        match self.table.try_find(key.hash, |(x, _)| x.equal(py, key))? {
            Some(x) => Ok(Some(unsafe { &x.as_ref().1 })),
            None => Ok(None),
        }
    }

    pub fn equal(&self, py: pyo3::Python<'_>, other: &Self) -> pyo3::PyResult<bool> {
        if self.maxsize != other.maxsize {
            return Ok(false);
        }

        if self.table.len() != other.table.len() {
            return Ok(false);
        }

        let mut error = None;

        let result = unsafe {
            self.table.iter().all(|bucket| {
                let (key, val) = bucket.as_ref();

                match other.table.try_find(key.hash, |(x, _)| x.equal(py, key)) {
                    Err(e) => {
                        error = Some(e);
                        true
                    }
                    Ok(Some(bucket)) => {
                        let (_, val2) = bucket.as_ref();

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
        self.observed.change();
    }

    pub fn shrink_to_fit(&mut self) {
        self.table.shrink_to(self.table.len(), |(x, _)| x.hash);
        self.observed.change();
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
                        entry.insert(hk, value.unbind())?;
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
                        entry.insert(hk, value)?;
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

        tuple!(check state, size=3)?;
        let (maxsize, iterable, capacity) = unsafe { extract_pickle_tuple!(py, state => dict) };

        let mut new = Self::new(maxsize, capacity)?;

        // SAFETY: we checked that the iterable is a dict in extract_pickle_tuple! macro
        let dict = unsafe {
            iterable
                .downcast_bound::<pyo3::types::PyDict>(py)
                .unwrap_unchecked()
        };

        unsafe {
            for (key, value) in dict.iter() {
                let hk = PreHashObject::from_pyobject(py, key.unbind()).unwrap_unchecked();

                match new.entry_with_slot(py, &hk)? {
                    Entry::Absent(entry) => {
                        entry.insert(hk, value.unbind())?;
                    }
                    _ => std::hint::unreachable_unchecked(),
                }
            }
        }

        *self = new;
        Ok(())
    }

    pub fn random_key(&self) -> Option<&PreHashObject> {
        if self.table.is_empty() {
            None
        } else {
            let nth = fastrand::usize(0..self.table.len());

            let bucket = unsafe { self.table.iter().nth(nth).unwrap_unchecked() };
            let (key, _) = unsafe { bucket.as_ref() };

            Some(key)
        }
    }
}

impl<'a> RandomPolicyOccupied<'a> {
    #[inline]
    pub fn update(self, value: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        unsafe {
            let old_value = std::mem::replace(&mut self.bucket.as_mut().1, value);

            // In update we don't need to change this; because this does not change the memory address ranges
            // self.instance.observed.change();

            Ok(old_value)
        }
    }

    #[inline]
    pub fn remove(self) -> (PreHashObject, pyo3::PyObject) {
        let (x, _) = unsafe { self.instance.table.remove(self.bucket) };
        self.instance.observed.change();
        x
    }

    pub fn into_value(self) -> &'a mut (PreHashObject, pyo3::PyObject) {
        unsafe { self.bucket.as_mut() }
    }
}

impl RandomPolicyAbsent<'_> {
    #[inline]
    pub fn insert(self, key: PreHashObject, value: pyo3::PyObject) -> pyo3::PyResult<()> {
        if self.instance.table.len() >= self.instance.maxsize.get() {
            self.instance.popitem()?;
        }

        match self.insert_slot {
            Some(slot) => unsafe {
                self.instance
                    .table
                    .insert_in_slot(key.hash, slot, (key, value));
            },
            None => {
                self.instance
                    .table
                    .insert(key.hash, (key, value), |(x, _)| x.hash);
            }
        }

        self.instance.observed.change();
        Ok(())
    }
}
