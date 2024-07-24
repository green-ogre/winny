#![allow(clippy::missing_safety_doc)]
use std::{cell::UnsafeCell, marker::PhantomData};

use crate::{
    ArchEntity, ArchId, ArchRow, Archetype, Archetypes, Bundle, Bundles, Column, ComponentId,
    ComponentMeta, Components, Entities, Entity, EntityMeta, MetaLocation, OwnedPtr, Res, ResMut,
    Resource, ResourceId, Resources, Table, TableId, TableRow, Tables, World,
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
        self.resources().id_unwrapped::<R>()
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

    pub unsafe fn take_resource_by_id<R: Resource>(self, id: ResourceId) -> Option<R> {
        self.world_mut().resources.take::<R>(id)
    }

    pub unsafe fn spawn_bundle<B: Bundle>(self, bundle: B) -> Entity {
        let _span = trace_span!("spawn bundle").entered();
        let bundle_meta = if let Some(meta) = self.world().bundles.meta::<B>() {
            meta
        } else {
            self.register_bundle::<B>();
            self.world().bundles.meta::<B>().unwrap()
        };
        trace!(bundle_meta = ?bundle_meta);

        // just registered
        let table = self
            .world_mut()
            .tables
            .get_mut_unchecked(bundle_meta.table_id);
        let mut bundle_components = bundle_meta.component_ids.iter();
        bundle.insert_components(&mut |component_ptr| {
            // bundle_components is the same order as bundle components
            if let Some(meta) = bundle_components.next() {
                trace!("pushing component ptr: {:?}", meta);
                // table is given by the registered bundle, therefore column of component type `id`
                // must exist
                table
                    .column_mut_unchecked(&meta.id)
                    .push_erased(component_ptr);
            }
        });

        // just registered
        let table_row = TableRow(self.tables().get_unchecked(bundle_meta.table_id).depth() - 1);
        let archetype = self
            .world_mut()
            .archetypes
            // just registered
            .get_mut_unchecked(bundle_meta.arch_id);

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

        let bundle_meta = if let Some(meta) = self.world().bundles.meta::<B>() {
            meta
        } else {
            self.register_bundle::<B>();
            self.world().bundles.meta::<B>().unwrap()
        };
        trace!(bundle_meta = ?bundle_meta);

        // just registered
        let table = self
            .world_mut()
            .tables
            .get_mut_unchecked(bundle_meta.table_id);
        let mut bundle_components = bundle_meta.component_ids.iter();
        bundle.insert_components(&mut |component_ptr| {
            // bundle_components is the same order as bundle components
            if let Some(meta) = bundle_components.next() {
                // table is given by the registered bundle, therefore column of component type `id`
                // must exist
                table
                    .column_mut_unchecked(&meta.id)
                    .push_erased(component_ptr);
            }
        });

        let table_row = TableRow(self.tables().get(bundle_meta.table_id).unwrap().depth() - 1);
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

    pub fn register_bundle<B: Bundle>(self) {
        let mut component_metas = Vec::new();
        B::component_meta(unsafe { self.components_mut() }, &mut |meta| {
            component_metas.push(*meta);
        });
        let unsorted_component_metas = component_metas.clone();
        component_metas.sort();
        let component_ids = component_metas.into_boxed_slice();

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
            self.world_mut().bundles.register::<B>(
                arch_id,
                table_id,
                unsorted_component_metas.into_boxed_slice(),
            )
        };
    }

    pub unsafe fn transfer_table_row(
        tables: &mut Tables,
        src_row: TableRow,
        src: TableId,
        dst: TableId,
    ) {
        Self::transfer_table_row_if(tables, src_row, src, dst, |_, _| true);
    }

    pub unsafe fn transfer_table_row_if<F>(
        tables: &mut Tables,
        src_row: TableRow,
        src: TableId,
        dst: TableId,
        f: F,
    ) where
        F: Fn(&ComponentId, &Column) -> bool,
    {
        let _span = trace_span!("conditional transfer table row").entered();

        let mut components: Vec<(ComponentId, OwnedPtr)> = Vec::with_capacity(10);
        {
            let src_table = tables.get_mut_unchecked(src);
            for (component_id, column) in src_table.iter_mut() {
                if f(component_id, column) {
                    let val = OwnedPtr::from(column.get_row_ptr_unchecked(src_row));
                    components.push((*component_id, val));
                }
            }
        }

        {
            let dst = tables.get_mut_unchecked(dst);
            for (component_id, ptr) in components.into_iter() {
                dst.column_mut_unchecked(&component_id).push_erased(ptr);
            }
        }

        {
            let src = tables.get_mut_unchecked(src);
            for (component_id, column) in src.iter_mut() {
                if f(component_id, column) {
                    column.swap_remove_row_no_drop(src_row);
                } else {
                    column.swap_remove_row_drop(src_row);
                }
            }
        }
    }

    pub unsafe fn insert_entity_into_world(
        entity: Entity,
        arch: &mut Archetype,
        tables: &mut Tables,
        entities: &mut Entities,
    ) {
        let _span = trace_span!("insert entity").entered();
        let new_table_row = unsafe { tables.get_unchecked(arch.table_id).depth() - 1 };
        let table_row = TableRow(new_table_row);
        let new_arch_entity = ArchEntity::new(entity, table_row);
        let arch_row = arch.new_entity(new_arch_entity);

        let new_location = MetaLocation {
            table_id: arch.table_id,
            archetype_id: arch.arch_id,
            arch_row,
            table_row,
        };

        entities.set_location(entity, new_location);
    }

    pub unsafe fn find_or_create_storage<B: Bundle>(
        meta: EntityMeta,
        component_metas: Box<[ComponentMeta]>,
        archetypes: &mut Archetypes,
        tables: &mut Tables,
        components: &mut Components,
        clone_if: &impl Fn(&ComponentId, &Column) -> bool,
        add_bundle_components: bool,
    ) -> (ArchId, TableId) {
        if let Some(arch) = archetypes.get_from_components(&component_metas) {
            trace!("archetype found: {:?}", arch.arch_id);
            (arch.arch_id, arch.table_id)
        } else {
            trace!("no archetype found, creating arch + table");
            let mut table = tables
                .get_unchecked(meta.location.table_id)
                .clone_empty_if(clone_if);
            if add_bundle_components {
                B::component_meta(components, &mut |meta| {
                    table.new_column_from_meta(meta);
                })
            }
            let table_id = tables.push(table);
            let arch_id = archetypes.push(Archetype::new(table_id, component_metas.clone()));

            (arch_id, table_id)
        }
    }
}
