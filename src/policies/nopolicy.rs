use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TryFindMethods;

pub struct NoPolicy {
    table: hashbrown::raw::RawTable<(PreHashObject, pyo3::PyObject)>,
    maxsize: std::num::NonZeroUsize,
    pub observed: Observed,
}

pub struct NoPolicyOccupied<'a> {
    instance: &'a mut NoPolicy,
    bucket: hashbrown::raw::Bucket<(PreHashObject, pyo3::PyObject)>,
}

pub struct NoPolicyAbsent<'a> {
    instance: &'a mut NoPolicy,
}

impl NoPolicy {
    #[inline]
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

    pub fn len(&self) -> usize {
        self.table.len()
    }

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

    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<NoPolicyOccupied, NoPolicyAbsent>> {
        match self.table.try_find(key.hash, |(x, _)| x.equal(py, key))? {
            Some(bucket) => {
                Ok(
                    Entry::Occupied(NoPolicyOccupied { instance: self, bucket })
                )
            },
            None => {
                Ok(
                    Entry::Absent(NoPolicyAbsent { instance: self })
                )
            }
        }
    }

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
    }

    pub fn shrink_to_fit(&mut self) {
        self.table.shrink_to(self.table.len(), |(x, _)| x.hash);
    }

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

                match self.entry(py, &hk)? {
                    Entry::Occupied(mut entry) => {
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

                match self.entry(py, &hk)? {
                    Entry::Occupied(mut entry) => {
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
        tuple!(check state, size=3)?;
        let (maxsize, iterable, capacity) = unsafe { extract_pickle_tuple!(py, state) };

        let mut new = Self::new(maxsize, capacity)?;
        new.extend(py, iterable)?;

        *self = new;
        Ok(())
    }
}

impl<'a> NoPolicyOccupied<'a> {
    pub fn update(&mut self, value: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        unsafe {
            let old_value = std::mem::replace(&mut self.bucket.as_mut().1, value);
            self.instance.observed.change();
            Ok(old_value)
        }
    }

    pub fn remove(self) -> (PreHashObject, pyo3::PyObject) {
        let (x, _) = unsafe { self.instance.table.remove(self.bucket) };
        x
    }

    pub fn into_value(self) -> &'a mut (PreHashObject, pyo3::PyObject) {
        unsafe { self.bucket.as_mut() }
    }
}

impl NoPolicyAbsent<'_> {
    pub fn insert(self, key: PreHashObject, value: pyo3::PyObject) -> pyo3::PyResult<()> {
        if self.instance.table.len() >= self.instance.maxsize.get() {
            // There's no algorithm for removing a key-value pair, so we raise PyOverflowError.
            return Err(pyo3::PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                "The cache has reached the bound",
            ));
        }

        self.instance
            .table
            .insert(key.hash, (key, value), |(x, _)| x.hash);

        self.instance.observed.change();
        Ok(())
    }
}
