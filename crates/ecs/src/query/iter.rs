use crate::{Archetypes, Tables};

use super::*;

pub struct QueryIter<'w, 's, T: QueryData, F: Filter> {
    world: UnsafeWorldCell<'w>,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    cursor: Cursor<'w, 's, T>,
    query_state: &'s QueryState<T, F>,
    filter: PhantomData<F>,
}

impl<'w, 's, T: QueryData, F: Filter> QueryIter<'w, 's, T, F> {
    pub fn new(world: UnsafeWorldCell<'w>, query_state: &'s QueryState<T, F>) -> Self {
        let cursor = Cursor::new(world, query_state);
        let tables = &unsafe { world.tables() };
        let archetypes = &unsafe { world.archetypes() };

        Self {
            query_state,
            world,
            tables,
            archetypes,
            cursor,
            filter: PhantomData,
        }
    }
}

impl<'w, 's, T: QueryData, F: Filter> Iterator for QueryIter<'w, 's, T, F> {
    type Item = T::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .next(self.tables, self.archetypes, self.query_state)
    }
}

struct Cursor<'w, 's, T: QueryData> {
    entities: &'w [ArchEntity],
    storage_ids: std::slice::Iter<'s, StorageId>,
    fetch: T::Fetch<'w>,
    current_row: usize,
    table_len: usize,
}

impl<'w, 's, T: QueryData> Cursor<'w, 's, T> {
    pub fn new<F: Filter>(world: UnsafeWorldCell<'w>, query_state: &'s QueryState<T, F>) -> Self {
        let fetch = T::init_fetch(world, &query_state.state);

        Cursor {
            fetch,
            storage_ids: query_state.storage_locations.iter(),
            entities: &[],
            current_row: 0,
            table_len: 0,
        }
    }

    pub fn next<F: Filter>(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<T, F>,
    ) -> Option<T::Item<'w>> {
        loop {
            if self.current_row == self.table_len {
                let storage_id = self.storage_ids.next()?;
                let table = tables.get(storage_id.table_id);
                let archetype = archetypes.get(&storage_id.archetype_id).unwrap();
                self.entities = archetype.entities.values();
                self.current_row = 0;
                self.table_len = table.depth();

                T::set_table(&mut self.fetch, &query_state.state, table);
                continue;
            }

            let res = T::fetch(&mut self.fetch, &self.entities[self.current_row]);
            self.current_row += 1;
            return Some(res);
        }
    }
}
