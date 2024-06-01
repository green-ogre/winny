use logger::warn;

use super::*;

// pub struct IndexAtlas {
//     components: SparseSet<TypeId, ComponentId>,
//     resources: SparseSet<TypeId, ResourceId>,
//     events: SparseSet<TypeId, EventId>,
// }

#[derive(Debug, Default)]
pub struct SparseArray<V> {
    values: Vec<Option<V>>,
}

impl<V: Debug> SparseArray<V> {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn get(&self, index: usize) -> Option<&V> {
        if index >= self.values.len() {
            None
        } else {
            self.values[index].as_ref()
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        if index >= self.values.len() {
            None
        } else {
            self.values[index].as_mut()
        }
    }

    pub fn insert(&mut self, index: usize, value: V) {
        while index >= self.values.len() {
            self.values.push(None);
        }

        // TODO: remove this if it is never proc
        if self.values[index].is_some() {
            warn!("Overwriting data stored in sparse array");
            println!("{:?}", value);
        }
        self.values[index] = Some(value);
    }

    pub fn insert_in_first_empty(&mut self, value: V) -> usize {
        let index = self
            .values
            .iter()
            .enumerate()
            .find(|(_, v)| v.is_none())
            .map(|(i, _)| i)
            .unwrap_or_else(|| {
                self.values.push(None);
                self.len() - 1
            });
        self.insert(index, value);

        index
    }

    pub fn remove(&mut self, index: usize) -> Option<V> {
        self.values[index].take()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

pub struct SparseArrayIter<'a, V> {
    sparse_array: &'a SparseArray<V>,
    cursor: usize,
}

impl<'a, V: Debug> Iterator for SparseArrayIter<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        while self.sparse_array.get(self.cursor).is_none() {
            if self.cursor == self.sparse_array.len() {
                return None;
            }
            self.cursor += 1;
        }

        self.sparse_array.get(self.cursor)
    }
}

// pub struct SparseArrayIntoIter<V> {
//     sparse_array: SparseArray<V>,
//     cursor: usize,
// }
//
// impl<I: SparseArrayIndex, V> SparseArrayIntoIter<I, V> {
//     pub fn new(sparse_array: SparseArray<I, V>) -> Self {
//         Self {
//             cursor: 0,
//             sparse_array,
//         }
//     }
// }
//
// impl<I: SparseArrayIndex, V> Iterator for SparseArrayIntoIter<I, V> {
//     type Item = V;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         while self.sparse_array.get(self.cursor).is_none() {
//             if self.cursor == self.values.len() {
//                 return None;
//             }
//             self.cursor += 1;
//         }
//
//         self.sparse_array.remove(self.cursor)
//     }
// }
//
// impl<I: SparseArrayIndex, V> IntoIterator for SparseArray<I, V> {
//     type Item = V;
//     type IntoIter = SparseArrayIntoIter<I, V>;
//
//     fn into_iter(self) -> Self::IntoIter {
//         SparseArrayIntoIter::new(self)
//     }
// }

#[derive(Debug, Default)]
pub struct SparseSet<I: SparseArrayIndex, V> {
    dense: Vec<V>,
    indexes: Vec<I>,
    sparse: SparseArray<usize>,
}

impl<I: SparseArrayIndex, V> SparseSet<I, V> {
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            indexes: Vec::new(),
            sparse: SparseArray::new(),
        }
    }

    pub fn insert(&mut self, index: I, value: V) {
        self.sparse.insert(index.to_index(), self.dense.len());
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
            .get(index.to_index())
            .map(|dense_index| &self.dense[*dense_index])
    }

    pub fn get_mut(&mut self, index: &I) -> Option<&mut V> {
        self.sparse
            .get(index.to_index())
            .map(|dense_index| &mut self.dense[*dense_index])
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

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn iter(&self) -> SparseSetIter<'_, I, V> {
        SparseSetIter::new(self)
    }

    // TODO: iter mut
    // pub fn iter_mut(&self) -> SparseSetIterMut<'_, I, V> {
    //     SparseSetIterMut::new(self)
    // }
}

pub struct SparseSetIter<'a, I: SparseArrayIndex, V> {
    sparse_set: &'a SparseSet<I, V>,
    indexes: std::slice::Iter<'a, I>,
}

impl<'a, I: SparseArrayIndex, V> SparseSetIter<'a, I, V> {
    pub fn new(sparse_set: &'a SparseSet<I, V>) -> Self {
        Self {
            indexes: sparse_set.indexes().iter(),
            sparse_set,
        }
    }
}

impl<'a, I: SparseArrayIndex, V> Iterator for SparseSetIter<'a, I, V> {
    type Item = (&'a I, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.indexes.next() {
            Some((index, self.sparse_set.get(index).unwrap()))
        } else {
            None
        }
    }
}

// pub struct SparseSetIterMut<'a, I: SparseArrayIndex, V> {
//     sparse_set: &'a mut SparseSet<I, V>,
//     indexes: std::slice::Iter<'a, I>,
// }
//
// impl<'a, I: SparseArrayIndex, V> SparseSetIterMut<'a, I, V> {
//     pub fn new(sparse_set: &'a mut SparseSet<I, V>) -> Self {
//         Self {
//             indexes: sparse_set.indexes().iter(),
//             sparse_set,
//         }
//     }
// }
//
// impl<'a, I: SparseArrayIndex, V> Iterator for SparseSetIterMut<'a, I, V> {
//     type Item = (&'a I, &'a mut V);
//
//     fn next(&mut self) -> Option<Self::Item> {
//         if let Some(index) = self.indexes.next() {
//             Some((index, self.sparse_set.get_mut(index).unwrap()))
//         } else {
//             None
//         }
//     }
// }
