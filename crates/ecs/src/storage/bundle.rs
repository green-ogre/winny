use std::any::TypeId;

use any_vec::AnyVec;

use util::tracing::{trace, warn};

use crate::{unsafe_world::UnsafeWorldCell, World};

use super::*;

pub trait Bundle: 'static + Send + Sync {
    fn push_storage(self, world: UnsafeWorldCell<'_>, table_id: TableId);
    fn new_table(self, world: &mut World) -> Table;
    fn register_components(&self, world: &mut World);
    fn type_ids(&self) -> Vec<TypeId>;
}

impl<T: Component + 'static> Bundle for T {
    fn push_storage(self, world: UnsafeWorldCell<'_>, table_id: TableId) {
        let component_id = unsafe { world.components() }.id(&std::any::TypeId::of::<T>());
        unsafe {
            world
                .tables_mut()
                .get_mut(table_id)
                .push_column(self, component_id);
        }
    }

    fn new_table(self, world: &mut World) -> Table {
        unsafe {
            let component_id = world.get_component_id(&std::any::TypeId::of::<T>());
            let mut column: AnyVec<dyn Send + Sync> = AnyVec::new::<T>();
            {
                let mut vec = column.downcast_mut_unchecked::<T>();
                vec.push(self);
            }

            let mut table = Table::new();
            table.insert_column(column, component_id);

            table
        }
    }

    fn register_components(&self, world: &mut World) {
        world.register_component::<T>();
    }

    fn type_ids(&self) -> Vec<TypeId> {
        vec![std::any::TypeId::of::<Self>()]
    }
}

macro_rules! bundle {
    ($($t:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($t: Bundle + 'static),*> Bundle for ($($t,)*) {

    fn push_storage(self, world: UnsafeWorldCell<'_>, table_id: TableId) {
               let ($($t,)*) = self;

                $(
                    $t.push_storage(world, table_id);
                )*
    }


    fn register_components(&self, world: &mut World) {
        $(
        world.register_component_by_id(std::any::TypeId::of::<$t>(), std::any::type_name::<$t>());
        )*
    }

            fn new_table(self, world: &mut World ) -> Table {
               let ($($t,)*) = self;
        unsafe {
            let mut table = Table::new();
       $(
            let component_id = world.get_component_id(&std::any::TypeId::of::<$t>());
            let mut column = AnyVec::new::<$t>();
            {
                let mut vec = column.downcast_mut_unchecked::<$t>();
                vec.push($t);
            }
            table.insert_column(column, component_id);
    )*


            table
        }
            }


            fn type_ids(&self) -> Vec<TypeId>  {
               let ($($t,)*) = self;

                vec![
                    $($t.type_ids(),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }

            // fn component_ids<'w>(&self, world: UnsafeWorldCell<'w>) -> Vec<ComponentId>  {
            //    let ($($t,)*) = self;
            //
            //     vec![
            //         $($t.component_ids(world),)*
            //     ].into_iter().flatten().collect::<Vec<_>>()
            // }
        }
    };

    ($(($t:ident)),*, $next:ident) => {
        bundle!($(($t)),*);
        bundle!($(($t)),*, $next);
    }
}

ecs_macro::all_tuples!(bundle, 1, 10, B);

#[derive(Default)]
pub struct Bundles {
    id_table: fxhash::FxHashMap<std::any::TypeId, BundleMeta>,
}

impl Bundles {
    pub fn register<B: Bundle>(&mut self, archetype: ArchId, table: TableId) -> BundleMeta {
        let id = std::any::TypeId::of::<B>();
        let meta = BundleMeta::new::<B>(archetype, table);

        trace!(
            "Registering bundle: {} => {:?}",
            std::any::type_name::<B>(),
            id
        );

        if let Some(old_meta) = self.id_table.insert(id, meta) {
            warn!(
                "Unnecessarily constructing new BundleMeta for [{:?}]",
                old_meta
            );
        }

        meta
    }

    pub fn get<B: Bundle>(&self) -> Option<&BundleMeta> {
        self.id_table.get(&std::any::TypeId::of::<B>())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BundleMeta {
    pub arch_id: ArchId,
    pub table_id: TableId,
}

impl BundleMeta {
    pub fn new<B: Bundle>(arch_id: ArchId, table_id: TableId) -> Self {
        Self { arch_id, table_id }
    }
}
