use super::*;
use crate::storage::DumbDrop;
use std::{alloc::Layout, any::TypeId};
use util::tracing::{error, trace};

// https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/component.rs#L189C1-L193C3
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Component`",
    label = "invalid `Component`",
    note = "consider annotating `{Self}` with `#[derive(Component)]`"
)]
#[cfg(not(target_arch = "wasm32"))]
pub trait Component: 'static + Send + Sync {}
#[cfg(target_arch = "wasm32")]
pub trait Component: 'static {}

#[derive(Default)]
pub struct Components {
    next_id: usize,
    type_id_table: fxhash::FxHashMap<TypeId, ComponentMeta>,
    component_id_table: fxhash::FxHashMap<ComponentId, ComponentMeta>,
}

impl Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Components")
            .field("next_id", &self.next_id)
            .field("type_id_table", &self.type_id_table)
            .finish_non_exhaustive()
    }
}

impl Components {
    pub fn register<T: Component>(&mut self) -> &ComponentMeta {
        let type_id = TypeId::of::<T>();
        if self.type_id_table.get(&type_id).is_none() {
            trace!("Registering component: {}", std::any::type_name::<T>(),);
            let id = self.new_id();
            let meta = ComponentMeta::new::<T>(id);
            self.type_id_table.insert(type_id, meta);
            self.component_id_table.insert(meta.id, meta);
        }

        // just created
        self.type_id_table.get(&type_id).unwrap()
    }

    pub fn meta<T: Component>(&self) -> &ComponentMeta {
        self.type_id_table
            .get(&std::any::TypeId::of::<T>())
            .unwrap()
    }

    pub fn meta_from_id(&self, id: ComponentId) -> Option<&ComponentMeta> {
        self.component_id_table.get(&id)
    }

    pub fn id(&self, type_id: &std::any::TypeId) -> ComponentId {
        let Some(meta) = self.type_id_table.get(type_id) else {
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentMeta {
    pub id: ComponentId,
    pub type_id: TypeId,
    pub drop: Option<DumbDrop>,
    pub name: &'static str,
    size: usize,
    align: usize,
}

impl Debug for ComponentMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentMeta")
            .field("id", &self.id)
            // .field("type_id", &self.type_id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl ComponentMeta {
    pub fn new<T: Component>(id: ComponentId) -> Self {
        let name = std::any::type_name::<T>();
        let drop = crate::storage::new_dumb_drop::<T>();
        let type_id = TypeId::of::<T>();
        let layout = std::alloc::Layout::new::<T>();
        let size = layout.size();
        let align = layout.align();

        Self {
            id,
            type_id,
            name,
            drop,
            size,
            align,
        }
    }

    pub fn layout(&self) -> Layout {
        unsafe { Layout::from_size_align_unchecked(self.size, self.align) }
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
