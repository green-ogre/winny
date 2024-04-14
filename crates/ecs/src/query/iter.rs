use super::*;

pub struct QueryIterStorage<'s> {
    storage: Vec<(&'s Archetype, &'s Table)>,
    table: &'s Table,
    archetype: &'s Archetype,
}

impl<'s> QueryIterStorage<'s> {
    pub fn new(storage: Vec<(&'s Archetype, &'s Table)>) -> Self {
        let (archetype, table) = storage.first().expect("cannot be empty");

        Self {
            table,
            archetype,
            storage,
        }
    }
}

pub struct QueryIter<'s, T, F> {
    cursor: std::slice::Iter<'s, ArchEntity>,
    storage: Option<QueryIterStorage<'s>>,
    next_storage: usize,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<'s, T, F> QueryIter<'s, T, F> {
    pub fn new(storage: Vec<(&'s Archetype, &'s Table)>) -> Self {
        if storage.first().is_none() {
            return Self::empty();
        }

        let storage = QueryIterStorage::new(storage);

        Self {
            cursor: storage.archetype.entities.iter(),
            next_storage: 1,
            storage: Some(storage),
            query: PhantomData,
            filter: PhantomData,
        }
    }

    fn empty() -> Self {
        Self {
            cursor: [].iter(),
            next_storage: 0,
            storage: None,
            query: PhantomData,
            filter: PhantomData,
        }
    }
}

impl<'s, T: QueryData, F> Iterator for QueryIter<'s, T, F> {
    type Item = T::ReadOnly<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.storage.is_none() {
            return None;
        }

        let Some(arch_entity) = self.cursor.next().or_else(|| {
            let storage = self.storage.as_mut().unwrap();

            let Some(next_storage) = storage.storage.get(self.next_storage) else {
                return None;
            };

            self.next_storage += 1;
            storage.archetype = &next_storage.0;
            storage.table = &next_storage.1;

            self.cursor = storage.archetype.entities.iter();
            let Some(next) = self.cursor.next() else {
                return None;
            };

            Some(next)
        }) else {
            return None;
        };

        Some(T::read_only(
            self.storage.as_ref().unwrap().table,
            arch_entity,
        ))
    }
}

pub struct QueryIterMut<'s, T, F> {
    cursor: std::slice::Iter<'s, ArchEntity>,
    storage: Option<QueryIterStorage<'s>>,
    next_storage: usize,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<'s, T, F> QueryIterMut<'s, T, F> {
    pub fn new(storage: Vec<(&'s Archetype, &'s Table)>) -> Self {
        if storage.first().is_none() {
            return Self::empty();
        }

        let storage = QueryIterStorage::new(storage);

        Self {
            cursor: storage.archetype.entities.iter(),
            next_storage: 1,
            storage: Some(storage),
            query: PhantomData,
            filter: PhantomData,
        }
    }

    fn empty() -> Self {
        Self {
            cursor: [].iter(),
            next_storage: 0,
            storage: None,
            query: PhantomData,
            filter: PhantomData,
        }
    }
}

impl<'s, T: QueryData, F> Iterator for QueryIterMut<'s, T, F> {
    type Item = T::Item<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.storage.is_none() {
            return None;
        }

        let Some(arch_entity) = self.cursor.next().or_else(|| {
            let storage = self.storage.as_mut().unwrap();

            let Some(next_storage) = storage.storage.get(self.next_storage) else {
                return None;
            };

            self.next_storage += 1;
            storage.archetype = &next_storage.0;
            storage.table = &next_storage.1;

            self.cursor = storage.archetype.entities.iter();
            let Some(next) = self.cursor.next() else {
                return None;
            };

            Some(next)
        }) else {
            return None;
        };

        Some(T::fetch(self.storage.as_ref().unwrap().table, arch_entity))
    }
}
