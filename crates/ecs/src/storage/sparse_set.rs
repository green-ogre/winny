use std::marker::PhantomData;

use util::tracing::warn;

use super::*;

pub trait SparseArrayIndex: Copy {
    fn index(&self) -> usize;
}

impl SparseArrayIndex for usize {
    fn index(&self) -> usize {
        *self
    }
}

#[derive(Debug)]
pub struct SparseArray<I: SparseArrayIndex, V> {
    values: Vec<Option<V>>,
    _phantom: PhantomData<I>,
}

impl<I: SparseArrayIndex, V> Default for SparseArray<I, V> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<I: SparseArrayIndex, V> SparseArray<I, V> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            _phantom: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: &I) -> Option<&V> {
        if index.index() >= self.values.len() {
            None
        } else {
            self.values[index.index()].as_ref()
        }
    }

    pub fn get_mut(&mut self, index: &I) -> Option<&mut V> {
        if index.index() >= self.values.len() {
            None
        } else {
            self.values[index.index()].as_mut()
        }
    }

    pub unsafe fn get_unchecked(&self, index: &I) -> &V {
        // TODO: remove when this works
        self.values.get(index.index()).unwrap().as_ref().unwrap()
    }

    pub unsafe fn get_mut_unchecked(&mut self, index: &I) -> &mut V {
        // TODO: remove when this works
        self.values
            .get_mut(index.index())
            .unwrap()
            .as_mut()
            .unwrap()
    }

    pub fn insert(&mut self, index: usize, value: V) {
        while index >= self.values.len() {
            self.values.push(None);
        }

        if std::mem::replace(&mut self.values[index], Some(value)).is_some() {
            warn!(
                "Overwriting data stored in ['SparseArray']: {}. This is unintented behaviour...",
                std::any::type_name::<V>()
            );
        }
    }

    pub fn insert_in_first_empty(&mut self, value: V) -> usize {
        let mut index = 0;
        for v in self.values.iter() {
            if v.is_none() {
                break;
            }
            index += 1;
        }

        if self.len() == index {
            self.values.push(Some(value));
            index
        } else {
            self.values[index] = Some(value);
            index
        }
    }

    pub fn push(&mut self, value: V) -> usize {
        self.values.push(Some(value));
        self.len() - 1
    }

    pub fn take(&mut self, index: usize) -> Option<V> {
        self.values[index].take()
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.values.iter().filter_map(|v| v.as_ref())
    }

    pub fn iter_indexed(&self) -> impl Iterator<Item = (usize, &V)> {
        self.values
            .iter()
            .enumerate()
            .filter(|(_, v)| v.as_ref().is_some())
            .map(|(i, v)| (i, unsafe { v.as_ref().unwrap_unchecked() }))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut().filter_map(|v| v.as_mut())
    }

    pub fn iter_indexed_mut(&mut self) -> impl Iterator<Item = (usize, &mut V)> {
        self.values
            .iter_mut()
            .enumerate()
            .filter(|(_, v)| v.as_ref().is_some())
            .map(|(i, v)| (i, unsafe { v.as_mut().unwrap_unchecked() }))
    }

    pub fn into_iter(self) -> impl Iterator<Item = V> {
        self.values.into_iter().filter_map(|f| f)
    }
}

#[derive(Debug)]
pub struct SparseSet<I: SparseArrayIndex, V> {
    dense: Vec<V>,
    indexes: Vec<I>,
    sparse: SparseArray<I, usize>,
}

impl<I: SparseArrayIndex, V> Default for SparseSet<I, V> {
    fn default() -> Self {
        Self {
            dense: Vec::new(),
            indexes: Vec::new(),
            sparse: SparseArray::default(),
        }
    }
}

#[allow(clippy::missing_safety_doc)]
impl<I: SparseArrayIndex, V> SparseSet<I, V> {
    // Required for AnyVec
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            indexes: Vec::new(),
            sparse: SparseArray::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            dense: Vec::with_capacity(cap),
            indexes: Vec::with_capacity(cap),
            sparse: SparseArray::with_capacity(cap),
        }
    }

    pub fn insert(&mut self, index: I, value: V) {
        self.sparse.insert(index.index(), self.dense.len());
        self.dense.push(value);
        self.indexes.push(index);
    }

    pub fn insert_in_first_empty(&mut self, value: V) -> usize {
        let dense_index = self.dense.len();
        self.dense.push(value);
        self.sparse.insert_in_first_empty(dense_index)
    }

    pub fn get(&self, index: &I) -> Option<&V> {
        self.sparse
            .get(index)
            .map(|dense_index| &self.dense[*dense_index])
    }

    pub fn get_mut(&mut self, index: &I) -> Option<&mut V> {
        self.sparse
            .get(index)
            .map(|dense_index| &mut self.dense[*dense_index])
    }

    pub unsafe fn get_unchecked(&self, index: &I) -> &V {
        let index = self.sparse.get_unchecked(index);
        &self.dense[*index]
    }

    pub unsafe fn get_mut_unchecked(&mut self, index: &I) -> &mut V {
        let index = self.sparse.get_unchecked(index);
        &mut self.dense[*index]
    }

    pub fn get_or_insert_with<F>(&mut self, index: I, f: F) -> &V
    where
        F: FnOnce() -> V,
    {
        if self.get(&index).is_none() {
            self.insert(index, f());
        }

        self.get(&index).unwrap()
    }

    pub fn remove(&mut self, index: &I) {
        for i in 0..self.indexes.len() {
            if self.indexes[i].index() == index.index() {
                self.indexes.remove(i);
                break;
            }
        }

        let Some(index) = self.sparse.get(index) else {
            panic!("removal index exceedes bounds");
        };

        let index = *index;

        self.dense.remove(index);
        self.sparse.take(index);
    }

    pub fn get_single(&self) -> Option<&V> {
        if self.dense.len() == 1 {
            Some(&self.dense[0])
        } else {
            None
        }
    }

    pub fn get_single_mut(&mut self) -> Option<&mut V> {
        if self.dense.len() == 1 {
            Some(&mut self.dense[0])
        } else {
            None
        }
    }

    pub fn indexes(&self) -> &[I] {
        &self.indexes
    }

    pub fn values(&self) -> &[V] {
        &self.dense
    }

    pub fn values_mut(&mut self) -> &mut [V] {
        &mut self.dense
    }

    pub fn contains_key(&self, index: &I) -> bool {
        self.get(index).is_some()
    }

    pub fn dense_len(&self) -> usize {
        self.dense.len()
    }

    pub fn sparse_len(&self) -> usize {
        self.sparse.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&I, &V)> {
        self.indexes.iter().zip(self.dense.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&I, &mut V)> {
        self.indexes.iter().zip(self.dense.iter_mut())
    }
}
