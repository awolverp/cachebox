use crate::common::Entry;
use crate::common::Observed;
use crate::common::PreHashObject;
use crate::common::TryFindMethods;
use crate::linked_list;

type NotNullNode = std::ptr::NonNull<linked_list::Node>;

pub struct LRUPolicy {
    table: hashbrown::raw::RawTable<NotNullNode>,
    list: linked_list::LinkedList,
    maxsize: std::num::NonZeroUsize,
    pub observed: Observed,
}

pub struct LRUPolicyOccupied<'a> {
    instance: &'a mut LRUPolicy,
    bucket: hashbrown::raw::Bucket<NotNullNode>,
}

pub struct LRUPolicyAbsent<'a> {
    instance: &'a mut LRUPolicy,
    insert_slot: Option<hashbrown::raw::InsertSlot>,
}

impl LRUPolicy {
    pub fn new(maxsize: usize, mut capacity: usize) -> pyo3::PyResult<Self> {
        let maxsize = non_zero_or!(maxsize, isize::MAX as usize);
        capacity = capacity.min(maxsize.get());

        Ok(Self {
            table: new_table!(capacity)?,
            list: linked_list::LinkedList::new(),
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
    pub fn popitem(&mut self) -> Option<(PreHashObject, pyo3::PyObject)> {
        let ret = self.list.head?;

        unsafe {
            self.table
                .remove_entry((*ret.as_ptr()).element.0.hash, |node| {
                    core::ptr::eq(node.as_ptr(), ret.as_ptr())
                })
                .expect("popitem key not found.");
        }

        self.observed.change();
        Some(self.list.pop_front().unwrap())
    }

    #[inline]
    #[rustfmt::skip]
    pub fn entry(
        &mut self,
        py: pyo3::Python<'_>,
        key: &PreHashObject,
    ) -> pyo3::PyResult<Entry<LRUPolicyOccupied, LRUPolicyAbsent>> {
        match self
            .table
            .try_find(key.hash, |x| unsafe { x.as_ref().element.0.equal(py, key) })?
        {
            Some(bucket) => {
                Ok(
                    Entry::Occupied(LRUPolicyOccupied { instance: self, bucket })
                )
            }
            None => {
                Ok(
                    Entry::Absent(LRUPolicyAbsent { instance: self, insert_slot: None })
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
    ) -> pyo3::PyResult<Entry<LRUPolicyOccupied, LRUPolicyAbsent>> {
        match self
            .table
            .try_find_or_find_insert_slot(
                key.hash,
                |x| unsafe { x.as_ref().element.0.equal(py, key) },
                |x| unsafe { x.as_ref().element.0.hash }
        )? {
            Ok(bucket) => {
                Ok(
                    Entry::Occupied(LRUPolicyOccupied { instance: self, bucket })
                )
            }
            Err(slot) => {
                Ok(
                    Entry::Absent(LRUPolicyAbsent { instance: self, insert_slot: Some(slot) })
                )
            },
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
                x.instance.list.move_back(*x.bucket.as_ptr());

                Ok(Some(&x.bucket.as_ref().as_ref().element.1))
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
            .try_find(key.hash, |x| unsafe { x.as_ref().element.0.equal(py, key) })?
            .map(|x| unsafe { &x.as_ref().as_ref().element.1 });

        Ok(result)
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.list.clear();
        self.observed.change();
    }

    pub fn shrink_to_fit(&mut self) {
        self.table
            .shrink_to(self.table.len(), |x| unsafe { x.as_ref().element.0.hash });

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
                let (key1, value1) = &node.as_ref().element;

                match other
                    .table
                    .try_find(key1.hash, |x| key1.equal(py, &x.as_ref().element.0))?
                {
                    Some(bucket) => {
                        let (_, value2) = &bucket.as_ref().as_ref().element;

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

    pub fn iter(&self) -> linked_list::Iter {
        self.list.iter()
    }

    pub fn least_recently_used(&self) -> Option<&(PreHashObject, pyo3::PyObject)> {
        self.list.head.map(|x| unsafe { &x.as_ref().element })
    }

    pub fn most_recently_used(&self) -> Option<&(PreHashObject, pyo3::PyObject)> {
        self.list.tail.map(|x| unsafe { &x.as_ref().element })
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

            let mut new = Self::new(maxsize, capacity)?;

            for pair in iterable.bind(py).try_iter()? {
                let (key, value) = pair?.extract::<(pyo3::PyObject, pyo3::PyObject)>()?;

                let hk = PreHashObject::from_pyobject(py, key)?;

                match new.entry_with_slot(py, &hk)? {
                    Entry::Absent(entry) => {
                        entry.insert(hk, value)?;
                    }
                    _ => std::hint::unreachable_unchecked(),
                }
            }

            *self = new;
            Ok(())
        }
    }
}

impl<'a> LRUPolicyOccupied<'a> {
    #[inline]
    pub fn update(self, value: pyo3::PyObject) -> pyo3::PyResult<pyo3::PyObject> {
        let item = unsafe { self.bucket.as_mut() };
        unsafe {
            self.instance.list.move_back(*item);
        }

        // In update we don't need to change this; because this does not change the memory address ranges
        // self.instance.observed.change();

        Ok(unsafe { std::mem::replace(&mut item.as_mut().element.1, value) })
    }

    #[inline]
    pub fn remove(self) -> (PreHashObject, pyo3::PyObject) {
        // let (PreHashObject { hash, .. }, _) = &self.instance.entries[self.index - self.instance.n_shifts];
        let (item, _) = unsafe { self.instance.table.remove(self.bucket) };
        let item = unsafe { self.instance.list.remove(item) };

        self.instance.observed.change();
        item
    }

    pub fn into_value(self) -> &'a mut (PreHashObject, pyo3::PyObject) {
        unsafe {
            self.instance.list.move_back(*self.bucket.as_ptr());
        }

        let item = unsafe { self.bucket.as_mut() };
        unsafe { &mut item.as_mut().element }
    }
}

impl LRUPolicyAbsent<'_> {
    #[inline]
    pub fn insert(self, key: PreHashObject, value: pyo3::PyObject) -> pyo3::PyResult<()> {
        if self.instance.table.len() >= self.instance.maxsize.get() {
            self.instance.popitem();
        }

        let hash = key.hash;
        let node = self.instance.list.push_back(key, value);

        match self.insert_slot {
            Some(slot) => unsafe {
                self.instance.table.insert_in_slot(hash, slot, node);
            },
            None => {
                self.instance
                    .table
                    .insert(hash, node, |x| unsafe { x.as_ref().element.0.hash });
            }
        }

        self.instance.observed.change();
        Ok(())
    }
}

unsafe impl Send for LRUPolicy {}
