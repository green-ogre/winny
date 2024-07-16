#![allow(clippy::missing_safety_doc)]
use std::{cell::UnsafeCell, marker::PhantomData};

use crate::{
    ArchEntity, ArchRow, Archetype, Archetypes, Bundle, BundleMeta, Bundles, Column, Component,
    ComponentId, Components, Entities, Entity, MetaLocation, OwnedPtr, Res, ResMut, Resource,
    ResourceId, Resources, Table, TableId, TableRow, Tables, World,
};

use util::tracing::{error, trace, trace_span};

// Useful for giving safe, multithreaded access to a ['World']
#[derive(Debug, Clone, Copy)]
pub struct UnsafeWorldCell<'w>(*mut World, PhantomData<(&'w World, &'w UnsafeCell<World>)>);

unsafe impl<'w> Send for UnsafeWorldCell<'w> {}
unsafe impl<'w> Sync for UnsafeWorldCell<'w> {}

impl<'w> UnsafeWorldCell<'w> {
    pub fn new(world: &'w World) -> Self {
        Self(std::ptr::from_ref(world).cast_mut(), PhantomData)
    }

    pub fn new_mut(world: &'w mut World) -> Self {
        Self(std::ptr::from_mut(world), PhantomData)
    }

    pub unsafe fn world(self) -> &'w World {
        &*self.0
    }

    pub unsafe fn world_mut(self) -> &'w mut World {
        &mut *self.0
    }

    pub unsafe fn tables(self) -> &'w Tables {
        &self.world().tables
    }

    pub unsafe fn tables_mut(self) -> &'w mut Tables {
        &mut self.world_mut().tables
    }

    pub unsafe fn archetypes(self) -> &'w Archetypes {
        &self.world().archetypes
    }

    pub unsafe fn archetypes_mut(self) -> &'w mut Archetypes {
        &mut self.world_mut().archetypes
    }

    pub unsafe fn resources(self) -> &'w Resources {
        &self.world().resources
    }

    pub unsafe fn components(self) -> &'w Components {
        &self.world().components
    }

    pub unsafe fn get_component_id(self, id: &std::any::TypeId) -> ComponentId {
        self.components().id(id)
    }

    pub unsafe fn register_component<C: Component>(self) -> ComponentId {
        self.components_mut().register::<C>()
    }

    pub unsafe fn register_component_by_id(
        self,
        id: std::any::TypeId,
        name: &'static str,
    ) -> ComponentId {
        self.components_mut().register_by_id(id, name)
    }

    pub unsafe fn get_component_ids(&self, ids: &[std::any::TypeId]) -> Vec<ComponentId> {
        let mut c_ids = Vec::new();
        for id in ids.iter() {
            c_ids.push(self.components().id(id))
        }

        c_ids
    }

    pub unsafe fn components_mut(self) -> &'w mut Components {
        &mut self.world_mut().components
    }

    pub unsafe fn entities(self) -> &'w Entities {
        &self.world().entities
    }

    pub unsafe fn entities_mut(self) -> &'w mut Entities {
        &mut self.world_mut().entities
    }

    pub unsafe fn bundles(self) -> &'w Bundles {
        &self.world().bundles
    }

    pub unsafe fn bundles_mut(self) -> &'w mut Bundles {
        &mut self.world_mut().bundles
    }

    pub unsafe fn get_resource_id<R: Resource>(self) -> ResourceId {
        self.resources().id::<R>().unwrap_or_else(|| {
            error!(
                "Resource ['{}'] is not registered. Remember to 'app.insert_resource::<R>()...'",
                std::any::type_name::<R>()
            );
            panic!("{} not registered", std::any::type_name::<R>());
        })
    }

    pub unsafe fn get_resource<R: Resource>(self) -> &'w R {
        let id = self.get_resource_id::<R>();
        self.get_resource_by_id(id)
    }

    pub unsafe fn get_resource_mut<R: Resource>(self) -> &'w mut R {
        let id = self.get_resource_id::<R>();
        self.get_resource_mut_by_id(id)
    }

    pub unsafe fn get_resource_ref<R: Resource>(self) -> Res<'w, R> {
        Res::new(self.get_resource::<R>())
    }

    pub unsafe fn get_resource_mut_ref<R: Resource>(self) -> ResMut<'w, R> {
        ResMut::new(self.get_resource_mut::<R>())
    }

    pub unsafe fn get_resource_ref_by_id<R: Resource>(self, id: ResourceId) -> Res<'w, R> {
        Res::new(self.get_resource_by_id::<R>(id))
    }

    pub unsafe fn get_resource_mut_ref_by_id<R: Resource>(self, id: ResourceId) -> ResMut<'w, R> {
        ResMut::new(self.get_resource_mut_by_id::<R>(id))
    }

    pub unsafe fn get_resource_by_id<R: Resource>(self, id: ResourceId) -> &'w R {
        self.try_get_resource_by_id(id).unwrap_or_else(|| {
            error!(
                "Resource ['{}'] is not in storage",
                std::any::type_name::<R>()
            );
            panic!();
        })
    }

    pub unsafe fn get_resource_mut_by_id<R: Resource>(self, id: ResourceId) -> &'w mut R {
        self.try_get_resource_mut_by_id(id).unwrap_or_else(|| {
            error!(
                "Resource ['{}'] is not in storage",
                std::any::type_name::<R>()
            );
            panic!();
        })
    }

    pub unsafe fn try_get_resource_by_id<R: Resource>(self, id: ResourceId) -> Option<&'w R> {
        self.resources().get_ptr::<R>(id).map(|ptr| ptr.as_ref())
    }

    pub unsafe fn try_get_resource_mut_by_id<R: Resource>(
        self,
        id: ResourceId,
    ) -> Option<&'w mut R> {
        self.resources()
            .get_ptr::<R>(id)
            .map(|mut ptr| ptr.as_mut())
    }

    pub unsafe fn try_get_resource_ref_by_id<R: Resource>(
        self,
        id: ResourceId,
    ) -> Option<Res<'w, R>> {
        self.resources()
            .get_ptr::<R>(id)
            .map(|ptr| Res::new(ptr.as_ref()))
    }

    pub unsafe fn try_get_resource_mut_ref_by_id<R: Resource>(
        self,
        id: ResourceId,
    ) -> Option<ResMut<'w, R>> {
        self.resources()
            .get_ptr::<R>(id)
            .map(|mut ptr| ResMut::new(ptr.as_mut()))
    }

    pub unsafe fn spawn_bundle<B: Bundle>(self, bundle: B) -> Entity {
        self.register_bundle::<B>();
        let bundle_meta = self
            .bundles_mut()
            .get_or_register_with(bundle, self, |bundle: B| {
                let type_ids = B::type_ids();
                let table = bundle.new_table(self);
                let table_id = self.tables_mut().push(table);
                let arch_id = self
                    .archetypes_mut()
                    .push(Archetype::new(table_id, type_ids));
                (table_id, arch_id)
            });
        trace!(bundle_meta = ?bundle_meta);

        let table_row = TableRow(self.tables().get(bundle_meta.table_id).unwrap().depth() - 1);
        let archetype = self.archetypes_mut().get_mut(bundle_meta.arch_id).unwrap();

        archetype.new_entity_with(table_row, |arch_row: ArchRow| {
            self.entities_mut().spawn(
                bundle_meta.table_id,
                bundle_meta.arch_id,
                table_row,
                arch_row,
            )
        })
    }

    pub unsafe fn spawn_bundle_with_entity<B: Bundle>(self, entity: Entity, bundle: B) {
        let _span = trace_span!("spawn bundle with entity", entity = ?entity).entered();
        self.register_bundle::<B>();
        // just created
        let bundle_meta = self.world().bundles.meta::<B>().unwrap();
        trace!(bundle_meta = ?bundle_meta);

        let (table, column) = bundle.insert_components(&mut |ptr| {});

        let table_row = TableRow(self.tables().get(bundle_meta.table_id).unwrap().depth());
        let archetype = self
            .world_mut()
            .archetypes
            .get_mut(bundle_meta.arch_id)
            .unwrap();

        archetype.new_entity_from(entity, table_row, |arch_index: ArchRow| {
            self.entities_mut().spawn_at(
                entity,
                bundle_meta.table_id,
                bundle_meta.arch_id,
                table_row,
                arch_index,
            )
        })
    }

    fn register_bundle<B: Bundle>(self) {
        let mut component_ids = Vec::new();
        B::component_meta(unsafe { self.components_mut() }, &mut |meta| {
            component_ids.push(meta.id);
        });
        component_ids.sort();
        let component_ids = component_ids.into_boxed_slice();

        let (arch_id, table_id);
        unsafe {
            if let Some(arch) = self.world().archetypes.get_from_components(&component_ids) {
                arch_id = arch.arch_id;
                table_id = arch.table_id;
            } else {
                let mut table = Table::default();
                B::component_meta(self.components_mut(), &mut |meta| {
                    table.new_column_from_meta(meta);
                });
                table_id = self.tables_mut().push(table);
                arch_id = self
                    .archetypes_mut()
                    .push(Archetype::new(table_id, component_ids.clone()));
            }
        }

        unsafe {
            self.world_mut()
                .bundles
                .register::<B>(arch_id, table_id, component_ids)
        };
    }

    pub unsafe fn transfer_table_row(self, src_row: TableRow, src: TableId, dst: TableId) {
        let _span = trace_span!("transfer table row").entered();
        let new_table = self.tables_mut().get_mut_unchecked(dst);
        let old_table = self.tables_mut().get_mut_unchecked(src);

        for (component_id, column) in old_table.iter_mut() {
            let val = OwnedPtr::from(column.get_row_ptr_unchecked(src_row));
            new_table
                .column_mut_unchecked(component_id)
                .push_erased(val);
            column.swap_remove_row_no_drop(src_row);
        }
    }

    pub unsafe fn transfer_table_row_if<F>(
        self,
        src_row: TableRow,
        src: TableId,
        dst: TableId,
        f: F,
    ) where
        F: Fn(&ComponentId, &Column) -> bool,
    {
        let _span = trace_span!("conditional transfer table row").entered();
        let new_table = self.tables_mut().get_mut_unchecked(dst);
        let old_table = self.tables_mut().get_mut_unchecked(src);

        for (component_id, column) in old_table.iter_mut() {
            if f(component_id, column) {
                let val = OwnedPtr::from(column.get_row_ptr_unchecked(src_row));
                new_table
                    .column_mut_unchecked(component_id)
                    .push_erased(val);
                column.swap_remove_row_no_drop(src_row);
            } else {
                column.swap_remove_row_drop(src_row);
            }
        }
    }

    pub unsafe fn insert_entity_into_world<F>(self, entity: Entity, arch: &mut Archetype, insert: F)
    where
        F: FnOnce(&Archetype),
    {
        // arch points to valid table
        let _span = trace_span!("insert entity").entered();
        let new_table_row = unsafe { self.tables().get_unchecked(arch.table_id).depth() };
        let table_row = TableRow(new_table_row);
        let new_arch_entity = ArchEntity::new(entity, table_row);
        let arch_row = arch.new_entity(new_arch_entity);

        insert(arch);

        let new_location = MetaLocation {
            table_id: arch.table_id,
            archetype_id: arch.arch_id,
            arch_row,
            table_row,
        };

        self.entities_mut().set_location(entity, new_location);
    }
}
