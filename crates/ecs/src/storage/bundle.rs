use std::any::TypeId;

use util::tracing::error;

use crate::unsafe_world::UnsafeWorldCell;

use super::*;

#[derive(Debug)]
pub enum IntoStorageError {
    LayoutMisMatch,
    ComponentIdMisMatch,
    IncorrectSparseIndex,
}

pub trait Bundle {
    fn new_storages<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<(ComponentId, DumbVec)>;
    fn push_storage<'w>(
        self,
        world: UnsafeWorldCell<'w>,
        table: &mut Table,
    ) -> Result<(), IntoStorageError>;
    fn type_ids(&self) -> Vec<TypeId>;
    fn component_ids<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<ComponentId>;
    fn storage_locations(&self) -> Vec<StorageType>;
}

impl<T: Storage + Component + 'static> Bundle for T {
    fn push_storage<'w>(
        self,
        world: UnsafeWorldCell<'w>,
        table: &mut Table,
    ) -> Result<(), IntoStorageError> {
        let ids = &self.type_ids();
        let component_id = *unsafe { world.read_only() }
            .get_component_ids(ids)
            .first()
            .unwrap();

        table.push_column(self, component_id).map_err(|err| {
            error!(
                "cached component id: {:?} => {:?}",
                ids.first().unwrap(),
                component_id
            );
            err
        })?;

        Ok(())
    }

    fn new_storages<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<(ComponentId, DumbVec)> {
        vec![(
            *unsafe { world.read_only() }
                .get_component_ids(&self.type_ids())
                .first()
                .unwrap(),
            DumbVec::new(std::alloc::Layout::new::<T>(), 1, new_dumb_drop::<T>()),
        )]
    }

    fn type_ids(&self) -> Vec<TypeId> {
        vec![std::any::TypeId::of::<Self>()]
    }

    fn component_ids<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<ComponentId> {
        vec![*unsafe { world.read_only() }
            .get_component_ids(&self.type_ids())
            .first()
            .unwrap()]
    }

    fn storage_locations(&self) -> Vec<StorageType> {
        vec![self.storage_type()]
    }
}

macro_rules! bundle {
    ($($t:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($t: Bundle),*> Bundle for ($($t,)*) {
            fn new_storages<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<(ComponentId, DumbVec)> {
               let ($($t,)*) = self;

                vec![
                    $($t.new_storages(world),)*
                ].into_iter().flatten().collect()
            }

            fn push_storage<'w>(self, world: UnsafeWorldCell<'w>, table: &mut Table) -> Result<(), IntoStorageError> {
               let ($($t,)*) = self;

                $(
                    $t.push_storage(world, table)?;
                )*

                Ok(())
            }

            fn type_ids(&self) -> Vec<TypeId>  {
               let ($($t,)*) = self;

                vec![
                    $($t.type_ids(),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }

            fn component_ids<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<ComponentId>  {
               let ($($t,)*) = self;

                vec![
                    $($t.component_ids(world),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }

            fn storage_locations(&self) -> Vec<StorageType> {
               let ($($t,)*) = self;

                vec![
                    $($t.storage_locations(),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }
        }
    };

    ($(($t:ident)),*, $next:ident) => {
        bundle!($(($t)),*);
        bundle!($(($t)),*, $next);
    }
}

ecs_macro::all_tuples!(bundle, 1, 10, B);
