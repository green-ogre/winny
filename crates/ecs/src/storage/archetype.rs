#![allow(clippy::missing_safety_doc)]
use std::{any::TypeId, marker::PhantomData};

use super::*;

use util::tracing::trace;

#[derive(Debug)]
pub struct Archetypes {
    archetypes: SparseSet<ArchId, Archetype>,
    id_table: fxhash::FxHashMap<Box<[ComponentId]>, ArchId>,
}

impl Default for Archetypes {
    fn default() -> Self {
        Self {
            archetypes: SparseSet::new(),
            id_table: fxhash::FxHashMap::default(),
        }
    }
}

impl Archetypes {
    pub fn push(&mut self, archetype: Archetype) -> ArchId {
        let arch_id = ArchId(self.archetypes.insert_in_first_empty(archetype));

        let archetype = unsafe { self.get_mut_unchecked(arch_id) };
        archetype.arch_id = arch_id;
        let ids = archetype.type_ids.clone();
        let _ = archetype;

        // self.id_table.insert(ids, arch_id);
        todo!();

        arch_id
    }

    pub fn len(&self) -> usize {
        self.archetypes.dense_len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, id: ArchId) -> Option<&Archetype> {
        self.archetypes.get(&id)
    }

    pub fn get_mut(&mut self, id: ArchId) -> Option<&mut Archetype> {
        self.archetypes.get_mut(&id)
    }

    pub unsafe fn get_unchecked(&self, id: ArchId) -> &Archetype {
        // Safety:
        // Cannot obtain a ['ArchId'] other than from Archetypes. Depends on the Immutability of ['Archetypes']
        self.archetypes.get_unchecked(&id)
    }

    pub unsafe fn get_mut_unchecked(&mut self, id: ArchId) -> &mut Archetype {
        // Safety:
        // Cannot obtain a ['ArchId'] other than from Archetypes. Depends on the Immutability of ['Archetypes']
        self.archetypes.get_mut_unchecked(&id)
    }

    pub fn get_from_components(&self, ids: &Box<[ComponentId]>) -> Option<&Archetype> {
        let arch_id = self.id_table.get(ids)?;
        self.archetypes.get(arch_id)
    }

    pub fn get_mut_from_components(&mut self, ids: Box<[ComponentId]>) -> Option<&mut Archetype> {
        let arch_id = self.id_table.get(&ids)?;
        self.archetypes.get_mut(arch_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ArchId, &Archetype)> {
        self.archetypes.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ArchId(usize);

impl ArchId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl SparseArrayIndex for ArchId {
    fn index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchRow(pub usize);

impl ArchRow {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
}

#[derive(Debug)]
pub struct ArchEntity {
    pub entity: Entity,
    pub row: TableRow,
}

impl ArchEntity {
    pub fn new(entity: Entity, row: TableRow) -> Self {
        Self { entity, row }
    }
}

#[derive(Debug)]
pub struct Archetype {
    pub arch_id: ArchId,
    pub table_id: TableId,
    pub entities: Vec<ArchEntity>,
    pub component_ids: Box<[ComponentId]>,
}

#[derive(Clone, Copy)]
pub struct ReservedArchRow<'r>(ArchRow, PhantomData<&'r ArchRow>);

impl<'r> Into<ArchRow> for ReservedArchRow<'r> {
    fn into(self) -> ArchRow {
        self.0
    }
}

#[derive(PartialEq, Eq)]
pub enum SwapEntity {
    Pop,
    Swap,
}

impl Archetype {
    pub fn new(table_id: TableId, component_ids: Box<[ComponentId]>) -> Self {
        Self {
            arch_id: ArchId::new(usize::MAX),
            entities: Vec::new(),
            table_id,
            component_ids,
        }
    }

    pub fn new_entity(&mut self, arch_entity: ArchEntity) -> ArchRow {
        let index = self.entities.len();
        self.entities.push(arch_entity);
        ArchRow::new(index)
    }

    pub fn get_entity(&mut self, arch_row: ArchRow) -> Option<Entity> {
        self.entities.get(arch_row.0).map(|a| a.entity)
    }

    pub unsafe fn get_entity_unchecked(&mut self, arch_row: ArchRow) -> Entity {
        self.entities.get_unchecked(arch_row.0).entity
    }

    pub fn swap_remove_entity(&mut self, arch_row: ArchRow) -> SwapEntity {
        let last_index = self.entities.len() - 1;
        if last_index != arch_row.0 {
            trace!(
                "swap_remove_entity: dst: {:?}, src: {:?}",
                arch_row,
                ArchRow(self.entities.len() - 1)
            );

            let _ = self.entities.swap_remove(arch_row.0);
            SwapEntity::Swap
        } else {
            trace!("swap_remove_entity: pop");

            self.entities.pop();
            SwapEntity::Pop
        }
    }

    pub fn new_entity_with<F>(&mut self, table_row: TableRow, f: F) -> Entity
    where
        F: FnOnce(ArchRow) -> Entity,
    {
        let index = self.reserve_entity();
        let entity = f(index.into());
        self.new_entity_reserved(ArchEntity::new(entity, table_row), index.into());
        entity
    }

    pub fn new_entity_from<F>(&mut self, entity: Entity, table_row: TableRow, f: F)
    where
        F: FnOnce(ArchRow),
    {
        let index = self.reserve_entity();
        f(index.into());
        self.new_entity_reserved(ArchEntity::new(entity, table_row), index.into());
    }

    // prevents having to call new_entity without first inserting entity into reserved index
    pub fn reserve_entity(&self) -> ReservedArchRow<'_> {
        ReservedArchRow(ArchRow::new(self.entities.len()), PhantomData)
    }

    pub fn new_entity_reserved(&mut self, arch_entity: ArchEntity, index: ArchRow) -> ArchRow {
        self.entities.insert(index.0, arch_entity);

        index
    }

    // pub fn contains_type_id<T: Component>(&self) -> bool {
    //     self.type_ids.contains(&TypeId::of::<T>())
    // }
    //
    // pub fn contains_query<T: QueryData>(&self) -> bool {
    //     self.contains_id_set(&T::set_ids())
    // }
    //
    // pub fn contains_id(&self, id: &TypeId) -> bool {
    //     self.type_ids.contains(id)
    // }
    //
    // pub fn contains_id_set(&self, components: &[TypeId]) -> bool {
    //     components.iter().all(|c| {
    //         if *c == std::any::TypeId::of::<Entity>() {
    //             true
    //         } else {
    //             self.type_ids.contains(c)
    //         }
    //     })
    // }
    //
    // pub fn comp_set_eq(&self, components: &[TypeId]) -> bool {
    //     components.iter().all(|c| self.type_ids.contains(c))
    //         && components.len() == self.type_ids.len()
    // }
}
