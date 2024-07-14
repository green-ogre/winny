pub mod commands;
pub mod entity;
pub mod unsafe_world;

use any_vec::AnyVec;
pub use commands::*;
pub use entity::*;

use std::any::TypeId;

use crate::{Archetype, Event, Events, Res, ResMut, Resource, Resources};

use crate::storage::*;

pub use self::unsafe_world::UnsafeWorldCell;

#[derive(Default)]
pub struct World {
    archetypes: Archetypes,
    tables: Tables,
    resources: Resources,
    components: Components,
    entities: Entities,
    bundles: Bundles,
}

impl World {
    pub unsafe fn as_unsafe_world(&self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new(self)
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let bundle_meta = if let Some(meta) = self.bundles.get::<B>() {
            bundle.push_storage(unsafe { self.as_unsafe_world() }, meta.table_id);
            meta
        } else {
            bundle.register_components(self);
            let type_ids = bundle.type_ids();
            let table = bundle.new_table(self);
            let table_id = self.tables.push(table);
            let arch_id = self.archetypes.push(Archetype::new(table_id, type_ids));
            &self.bundles.register::<B>(arch_id, table_id)
        };

        let table_row = TableRow(self.tables.get(bundle_meta.table_id).depth());
        let archetype = self.archetypes.get_mut(&bundle_meta.arch_id);

        archetype.new_entity_with(table_row, |arch_index: ArchIndex| {
            self.entities
                .spawn(bundle_meta.table_id, bundle_meta.arch_id, arch_index)
        })
    }

    pub fn spawn_with_entity<B: Bundle>(&mut self, entity: Entity, bundle: B) {
        let bundle_meta = if let Some(meta) = self.bundles.get::<B>() {
            bundle.push_storage(unsafe { self.as_unsafe_world() }, meta.table_id);
            meta
        } else {
            bundle.register_components(self);
            let type_ids = bundle.type_ids();
            let table = bundle.new_table(self);
            let table_id = self.tables.push(table);
            let arch_id = self.archetypes.push(Archetype::new(table_id, type_ids));
            &self.bundles.register::<B>(arch_id, table_id)
        };

        let table_row = TableRow(self.tables.get(bundle_meta.table_id).depth());
        let archetype = self.archetypes.get_mut(&bundle_meta.arch_id);

        archetype.new_entity_from(entity, table_row, |arch_index: ArchIndex| {
            self.entities.spawn_at(
                entity,
                bundle_meta.table_id,
                bundle_meta.arch_id,
                arch_index,
            )
        })
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.entities.despawn(entity);
        self.remove_entity(entity)
    }

    fn remove_entity(&mut self, _entity: Entity) {
        todo!();
        // let meta = self.get_entity(entity).ok_or(())?;

        // self.tables[meta.location.table_id].len -= 1;

        // let changed_table_row = TableRow(self.tables[meta.location.table_id].len);
        // for v in self.tables[meta.location.table_id].storage.iter_mut() {
        //     v.swap_remove(meta.location.table_row.0)?;
        // }

        // if changed_table_row == meta.location.table_row {
        //     self.archetypes[meta.location.archetype_id]
        //         .entities
        //         .swap_remove(meta.location.archetype_index);
        //     return Ok(());
        // }

        // if let Some(moved_entity) = self.archetypes[meta.location.archetype_id]
        //     .entities
        //     .iter()
        //     .rev()
        //     .find(|a_entity| a_entity.row == changed_table_row)
        //     .map(|a_entity| a_entity.entity)
        // {
        //     let changed_meta = self.get_entity_mut(moved_entity).ok_or(())?;
        //     changed_meta.location = meta.location;
        // } else if let Some(moved_entity) = self
        //     .archetypes
        //     .iter()
        //     .filter(|arch| arch.table_id == meta.location.table_id)
        //     .map(|arch| {
        //         arch.entities
        //             .iter()
        //             .rev()
        //             .find(|a_entity| a_entity.row == changed_table_row)
        //             .map(|a_entity| a_entity.entity)
        //     })
        //     .exactly_one()
        //     .map_err(|_| ())?
        // {
        //     let changed_meta = self.get_entity_mut(moved_entity).ok_or(())?;
        //     changed_meta.location.table_row = changed_table_row;
        // }

        // self.archetypes[meta.location.archetype_id]
        //     .entities
        //     .swap_remove(meta.location.archetype_index);

        // self.archetypes[meta.location.archetype_id].entities[meta.location.archetype_index].row =
        //     meta.location.table_row;
    }

    pub fn apply_entity_commands(
        &mut self,
        _entity: Entity,
        _new_ids: Vec<TypeId>,
        _remove_ids: Vec<TypeId>,
        _insert_ids: Vec<TypeId>,
    ) {
        // // Checked by commands before called
        // let entity_meta = self.get_entity(entity).unwrap();

        // self.register_components(&new_ids);

        // let remove_component_ids = self.get_component_ids(&remove_ids);
        // let component_ids = self.get_component_ids(&new_ids);
        // let current_table = unsafe { self.as_unsafe_world().read_and_write() }
        //     .tables
        //     .get_mut(&entity_meta.location.table_id)
        //     .unwrap();

        // let current_table_row = self
        //     .archetypes
        //     .get(&entity_meta.location.archetype_id)
        //     .unwrap()
        //     .get_entity_table_row(entity_meta);

        // let meta = if let Some(arch) = self
        //     .archetypes
        //     .get_from_type_ids(new_ids.clone().as_mut_slice())
        // {
        //     for (component_id, type_id) in component_ids.iter().zip(new_ids.iter()) {
        //         remove_entity_from_old_storage_and_put_in_new(
        //             {
        //                 if insert_ids.contains(type_id) {
        //                     &mut insert
        //                         .iter_mut()
        //                         .find(|i| i.type_id == *type_id)
        //                         .unwrap()
        //                         .component
        //                 } else {
        //                     current_table.column_mut(component_id).unwrap()
        //                 }
        //             },
        //             {
        //                 let table = self.tables.get_mut(&arch.table_id).expect("must exist");
        //                 table.column_mut(component_id).unwrap()
        //             },
        //             current_table_row.0,
        //         );
        //     }

        //     for _component_id in remove_component_ids.iter() {
        //         todo!();
        //     }

        //     let table_row = TableRow(
        //         self.tables
        //             .get(&arch.table_id)
        //             .expect("must exist")
        //             .depth()
        //             .saturating_sub(1),
        //     );
        //     let arch_index = arch.new_entity(ArchEntity::new(entity, table_row));

        //     MetaLocation::new(arch.table_id, arch.id, arch_index)
        // } else {
        //     let arch_id = self.archetypes.new_id();
        //     let mut storages = current_table
        //         .iter()
        //         .filter(|(id, _)| component_ids.contains(id))
        //         .map(|(id, c)| (*id, c.clone_empty()))
        //         .collect::<Vec<_>>();
        //     storages.extend(
        //         insert
        //             .into_iter()
        //             .map(|i| (self.get_component_ids(&[i.type_id])[0], i.component)),
        //     );

        //     for (component_id, type_id) in component_ids.iter().zip(new_ids.iter()) {
        //         if !insert_ids.contains(type_id) {
        //             remove_entity_from_old_storage_and_put_in_new(
        //                 current_table.column_mut(component_id).unwrap(),
        //                 {
        //                     storages
        //                         .iter_mut()
        //                         .find(|(id, _)| *id == *component_id)
        //                         .map(|(_, vec)| vec)
        //                         .unwrap()
        //                 },
        //                 current_table_row.0,
        //             );
        //         }
        //     }

        //     let mut table = Table::new();
        //     for (id, column) in storages.into_iter() {
        //         unsafe { table.insert_column(column, id) };
        //     }

        //     let table_id = self.tables.push(table);
        //     self.archetypes
        //         .new_archetype(arch_id, Archetype::new(arch_id, table_id, new_ids));

        //     let arch = self.archetypes.get_mut(&arch_id);
        //     let table_row = TableRow(0);
        //     let arch_index = arch.new_entity(ArchEntity::new(entity, table_row));

        //     MetaLocation::new(arch.table_id, arch.id, arch_index)
        // };

        // self.archetypes
        //     .get_mut(&entity_meta.location.archetype_id)
        //     .remove_entity(entity_meta.location.archetype_index);

        // self.entities[entity.index() as usize].location = meta;
    }

    pub fn register_resource<R: Resource>(&mut self) -> ResourceId {
        self.resources.register::<R>()
    }

    pub fn insert_resource<R: Resource>(&mut self, res: R) {
        let id = self.register_resource::<R>();
        self.resources.insert(res, id);
    }

    pub fn get_resource_id<R: Resource>(&self) -> ResourceId {
        unsafe { self.as_unsafe_world().get_resource_id::<R>() }
    }

    pub fn resource<R: Resource>(&self) -> Res<R> {
        unsafe { self.as_unsafe_world().get_resource_ref::<R>() }
    }

    pub fn resource_mut<R: Resource>(&mut self) -> ResMut<R> {
        unsafe { self.as_unsafe_world().get_resource_mut_ref::<R>() }
    }

    pub fn register_component<C: Component>(&mut self) -> ComponentId {
        self.components.register::<C>()
    }

    pub fn register_component_by_id(
        &mut self,
        id: std::any::TypeId,
        name: &'static str,
    ) -> ComponentId {
        self.components.register_by_id(id, name)
    }

    pub fn get_component_id(&self, id: &std::any::TypeId) -> ComponentId {
        self.components.id(id)
    }

    pub fn register_event<E: Event>(&mut self) {
        self.insert_resource(Events::<E>::new());
    }

    pub fn push_event<E: Event>(&mut self, event: E) {
        let mut events = self.resource_mut::<Events<E>>();
        events.push(event);
    }

    pub fn push_event_queue<E: Event>(&mut self, event_queue: Vec<E>) {
        let mut events = self.resource_mut::<Events<E>>();
        events.append(event_queue.into_iter());
    }

    pub fn entity(&self, entity: Entity) -> EntityRef<'_> {
        EntityRef::new(self, entity)
    }

    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut<'_> {
        EntityMut::new(self, entity)
    }
}

fn remove_entity_from_old_storage_and_put_in_new(src: &mut AnyVec, dst: &mut AnyVec, index: usize) {
    let old = src.remove(index);
    dst.push(old);
}
