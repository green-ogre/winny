use logger::error;

use super::*;

pub struct QueryState<T, F> {
    storages: Vec<StorageId>,
    component_access: Vec<ComponentAccess>,
    component_ids: Vec<ComponentId>,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<T, F> Debug for QueryState<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryState")
            .field("query", &self.query)
            .field("filter", &self.filter)
            .field("storages", &self.storages)
            .field("component_access", &self.component_access)
            .finish()
    }
}

impl<T: QueryData, F: Filter> QueryState<T, F> {
    pub fn from_world_unsafe<'w>(world: UnsafeWorldCell<'w>) -> Self {
        let storages = unsafe {
            world
                .read_only()
                .archetypes
                .iter()
                .filter(|arch| arch.contains_query::<T>())
                .filter(|arch| F::condition(arch))
                .map(|arch| StorageId {
                    table_id: arch.table_id,
                    archetype_id: arch.id,
                })
                .collect()
        };

        let component_ids = unsafe { world.read_only() }.get_component_ids(&T::set_ids());

        Self::new(storages, component_ids)
    }

    pub fn new(storages: Vec<StorageId>, component_ids: Vec<ComponentId>) -> Self {
        Self {
            storages,
            component_ids,
            component_access: T::set_access(),
            query: PhantomData,
            filter: PhantomData,
        }
    }

    pub fn component_access(&self) -> Vec<ComponentAccess> {
        self.component_access.clone()
    }

    pub fn iter_component_ids(&self) -> impl Iterator<Item = ComponentId> {
        self.component_ids.clone().into_iter()
    }

    pub fn new_iter(&self, world: &UnsafeWorldCell<'_>) -> QueryIter<'_, T, F> {
        let storage: Vec<_> = self
            .storages
            .iter()
            .map(|id| unsafe {
                (
                    world
                        .read_only()
                        .archetypes
                        .get(id.archetype_id)
                        .expect("correct index"),
                    world
                        .read_only()
                        .tables
                        .get(id.table_id)
                        .expect("correct index"),
                )
            })
            .collect();

        QueryIter::new(self, storage)
    }

    pub fn new_iter_mut<'w>(&self, world: &'w UnsafeWorldCell<'w>) -> QueryIterMut<'_, T, F> {
        let storage: Vec<_> = self
            .storages
            .iter()
            .map(|id| unsafe {
                (
                    world
                        .read_only()
                        .archetypes
                        .get(id.archetype_id)
                        .expect("correct index"),
                    world
                        .read_only()
                        .tables
                        .get(id.table_id)
                        .expect("correct index"),
                )
            })
            .collect();

        QueryIterMut::new(self, storage)
    }

    pub fn get_single<'w>(&self, world: &'w UnsafeWorldCell) -> Result<T::ReadOnly<'_>, ()> {
        Ok(T::read_only(
            unsafe { world.read_only() }
                .tables
                .get(
                    self.storages
                        .first()
                        .ok_or_else(|| {
                            error!("Query could not find a table for single");
                        })?
                        .table_id,
                )
                .expect("correct table id"),
            unsafe { world.read_only() }
                .archetypes
                .get(
                    self.storages
                        .first()
                        .ok_or_else(|| {
                            error!("Query could not find an archetype for single");
                        })?
                        .archetype_id,
                )
                .expect("correct archetype id")
                .entities
                .get_single()
                .ok_or_else(|| {
                    error!("Query could not produce any entities for single");
                    ()
                })?,
            self.iter_component_ids(),
        ))
    }

    // TODO: error handling
    pub fn get_single_mut<'w>(&self, world: &'w UnsafeWorldCell) -> Result<T::Item<'_>, ()> {
        Ok(T::fetch(
            unsafe { world.read_only() }
                .tables
                .get(
                    self.storages
                        .first()
                        .ok_or_else(|| {
                            error!("Query could not find a table for single mut");
                        })?
                        .table_id,
                )
                .expect("correct table id"),
            unsafe { world.read_and_write() }
                .archetypes
                .get_mut(
                    self.storages
                        .first()
                        .ok_or_else(|| {
                            error!("Query could not find an archetype for single mut");
                        })?
                        .archetype_id,
                )
                .entities
                .get_single_mut()
                .ok_or_else(|| {
                    error!("Query could not produce any entities for single mut");
                    ()
                })?,
            self.iter_component_ids(),
        ))
    }
}
