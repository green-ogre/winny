use super::*;

// TODO: this can easily be cached in the world with an id
pub struct QueryState<T, F> {
    storages: Vec<StorageId>,
    component_access: Vec<ComponentAccess>,
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
    pub fn from_world(world: &World) -> Self {
        let storages = unsafe {
            world
                .as_unsafe_world()
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

        Self::new(storages)
    }

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

        Self::new(storages)
    }

    pub fn new(storages: Vec<StorageId>) -> Self {
        Self {
            storages,
            component_access: T::set_access(),
            query: PhantomData,
            filter: PhantomData,
        }
    }

    pub fn component_access(&self) -> Vec<ComponentAccess> {
        self.component_access.clone()
    }

    pub fn new_iter<'w>(&self, world: &'w UnsafeWorldCell<'w>) -> QueryIter<'_, T, F> {
        let storage: Vec<_> = self
            .storages
            .iter()
            .map(|id| unsafe {
                (
                    world.read_only().archetypes.get(id.archetype_id),
                    world.read_only().tables.get(id.table_id),
                )
            })
            .collect();

        QueryIter::new(storage)
    }

    pub fn new_iter_mut<'w>(&self, world: &'w UnsafeWorldCell<'w>) -> QueryIterMut<'_, T, F> {
        let storage: Vec<_> = self
            .storages
            .iter()
            .map(|id| unsafe {
                (
                    world.read_only().archetypes.get(id.archetype_id),
                    world.read_only().tables.get(id.table_id),
                )
            })
            .collect();

        QueryIterMut::new(storage)
    }

    // TODO: error handling
    pub fn get_single<'w>(&self, world: &'w UnsafeWorldCell) -> anyhow::Result<T::ReadOnly<'_>> {
        Ok(T::read_only(
            unsafe { world.read_only() }
                .tables
                .get(self.storages.first().unwrap().table_id),
            unsafe { world.read_only() }
                .archetypes
                .get(self.storages.first().unwrap().archetype_id)
                .entities
                .first()
                .unwrap(),
        ))
    }

    // TODO: error handling
    pub fn get_single_mut<'w>(&self, world: &'w UnsafeWorldCell) -> anyhow::Result<T::Item<'_>> {
        Ok(T::fetch(
            unsafe { world.read_only() }
                .tables
                .get(self.storages.first().unwrap().table_id),
            unsafe { world.read_only() }
                .archetypes
                .get(self.storages.first().unwrap().archetype_id)
                .entities
                .first()
                .unwrap(),
        ))
    }
}
