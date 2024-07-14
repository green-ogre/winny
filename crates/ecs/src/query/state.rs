use super::*;

pub struct QueryState<T: QueryData, F = ()> {
    pub storage_locations: Vec<StorageId>,
    system_access: SystemAccess,
    pub state: T::State,
    filter: PhantomData<F>,
}

unsafe impl<T: QueryData, F: Filter> Send for QueryState<T, F> {}
unsafe impl<T: QueryData, F: Filter> Sync for QueryState<T, F> {}

impl<T: QueryData, F: Filter> Debug for QueryState<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryState")
            .field("filter", &self.filter)
            .finish()
    }
}

impl<T: QueryData, F: Filter> QueryState<T, F> {
    pub fn from_world_unsafe(world: UnsafeWorldCell<'_>) -> Self {
        let storages = unsafe {
            world
                .archetypes()
                .iter()
                .filter(|(_, arch)| arch.contains_query::<T>())
                .filter(|(_, arch)| F::condition(arch))
                .map(|(id, arch)| StorageId {
                    table_id: arch.table_id,
                    archetype_id: *id,
                })
                .collect()
        };
        Self::new(world, storages)
    }

    pub fn new(world: UnsafeWorldCell<'_>, storage_locations: Vec<StorageId>) -> Self {
        let state = T::init_state(world);

        Self {
            storage_locations,
            system_access: T::system_access(unsafe { world.components_mut() }),
            state,
            filter: PhantomData,
        }
    }

    pub fn system_access(&self) -> SystemAccess {
        todo!();
    }

    // From https://github.com/bevyengine/bevy/blob/d7080369a7471e6aa9747bad41a4469092f9967b/crates/bevy_ecs/src/query/state.rs#L124
    fn as_transmuted_state<NewD: QueryData, NewF: Filter>(&self) -> &QueryState<NewD, NewF> {
        unsafe { &*std::ptr::from_ref(self).cast::<QueryState<NewD, NewF>>() }
    }

    pub fn read_only(&self) -> &QueryState<T::ReadOnly, F> {
        self.as_transmuted_state::<T::ReadOnly, F>()
    }

    pub fn new_iter<'w>(&self, world: UnsafeWorldCell<'w>) -> QueryIter<'w, '_, T, F> {
        QueryIter::new(world, self)
    }

    pub fn get_single<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
    ) -> Result<T::Item<'w>, SingleQueryError> {
        let mut iter = self.new_iter(world);
        let Some(first) = iter.next() else {
            return Err(SingleQueryError::None);
        };
        if iter.next().is_some() {
            Err(SingleQueryError::Many)
        } else {
            Ok(first)
        }
    }
}

#[derive(Debug)]
pub enum SingleQueryError {
    None,
    Many,
}

impl std::fmt::Display for SingleQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SingleQueryError::None => {
                write!(f, "get single query produced no result")
            }
            SingleQueryError::Many => {
                write!(f, "get single query produced more than one result")
            }
        }
    }
}

impl std::error::Error for SingleQueryError {}
