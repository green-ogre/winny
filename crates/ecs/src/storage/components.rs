use std::{alloc::Layout, any::TypeId};

use crate::storage::DumbDrop;

use super::*;

use util::tracing::{error, trace};

// https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/component.rs#L189C1-L193C3
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Component`",
    label = "invalid `Component`",
    note = "consider annotating `{Self}` with `#[derive(Component)]`"
)]
pub trait Component: 'static + Send + Sync {}

#[derive(Debug, Default)]
pub struct Components {
    next_id: usize,
    id_table: fxhash::FxHashMap<std::any::TypeId, ComponentMeta>,
}

impl Components {
    pub fn register<T: Component>(&mut self) -> &ComponentMeta {
        let type_id = std::any::TypeId::of::<T>();
        if self.id_table.get(&type_id).is_none() {
            let id = self.new_id();
            let meta = ComponentMeta::new::<T>(id);
            self.id_table.insert(type_id, meta);

            trace!("Registering component: {}", std::any::type_name::<T>(),);
        }

        // just created
        self.id_table.get(&type_id).unwrap()
    }

    // pub fn register_by_id(&mut self, type_id: std::any::TypeId, name: &'static str) -> ComponentId {
    //     if let Some(meta) = self.id_table.get(&type_id) {
    //         meta.id
    //     } else {
    //         let id = self.new_id();
    //         let meta = ComponentMeta { id, name };
    //         self.id_table.insert(type_id, meta);
    //
    //         trace!("Registering component by id: {}", name);
    //
    //         id
    //     }
    // }

    pub fn meta<T: Component>(&self) -> &ComponentMeta {
        self.id_table.get(&std::any::TypeId::of::<T>()).unwrap()
    }

    pub fn id(&self, type_id: &std::any::TypeId) -> ComponentId {
        let Some(meta) = self.id_table.get(type_id) else {
            error!("Failed to get component meta: {:?}", type_id);
            panic!();
        };

        meta.id
    }

    pub fn ids(&self, type_ids: &[std::any::TypeId]) -> Vec<ComponentId> {
        let mut component_ids = Vec::with_capacity(type_ids.len());
        for t in type_ids.iter() {
            component_ids.push(self.id(t));
        }

        component_ids
    }

    fn new_id(&mut self) -> ComponentId {
        let id = self.next_id;
        self.next_id += 1;

        ComponentId::new(id)
    }
}

#[derive(Debug)]
pub struct ComponentMeta {
    pub id: ComponentId,
    pub layout: Layout,
    pub drop: Option<DumbDrop>,
    pub name: &'static str,
}

impl ComponentMeta {
    pub fn new<T: Component>(id: ComponentId) -> Self {
        let name = std::any::type_name::<T>();
        let layout = std::alloc::Layout::new::<T>();
        let drop = crate::storage::new_dumb_drop::<T>();

        Self {
            id,
            name,
            layout,
            drop,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Copy)]
pub struct ComponentId(usize);

impl ComponentId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl SparseArrayIndex for ComponentId {
    fn index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComponentSet {
    pub ids: Vec<TypeId>,
}

impl ComponentSet {
    pub fn new(mut ids: Vec<TypeId>) -> Self {
        // Assume that Entity is the first Component?
        ids.insert(0, std::any::TypeId::of::<Entity>());
        Self { ids }
    }

    pub fn contains<T: Component>(&self) -> bool {
        self.ids.contains(&TypeId::of::<T>())
    }

    pub fn contains_id(&self, id: &TypeId) -> bool {
        self.ids.contains(id)
    }

    pub fn equivalent(&self, components: &[TypeId]) -> bool {
        self.ids.eq(components)
    }
}
