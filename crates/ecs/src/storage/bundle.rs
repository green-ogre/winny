use util::tracing::{trace, warn};

use super::*;

// https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/bundle.rs#L146C1-L150C3
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Bundle`",
    label = "invalid `Bundle`",
    note = "consider annotating `{Self}` with `#[derive(Component)]` or `#[derive(Bundle)]`"
)]
pub trait Bundle: 'static + Send + Sync {
    fn component_meta<F: FnMut(&ComponentMeta)>(components: &mut Components, ids: &mut F);
    // Inserted in the order of [`component_ids`]
    fn insert_components<F: FnMut(OwnedPtr)>(self, f: &mut F);
}

impl<C: Component> Bundle for C {
    fn component_meta<F: FnMut(&ComponentMeta)>(components: &mut Components, ids: &mut F) {
        ids(components.register::<C>())
    }
    // Inserted in the order of [`component_ids`]
    fn insert_components<F: FnMut(OwnedPtr)>(self, f: &mut F) {
        OwnedPtr::make(self, |self_ptr| f(self_ptr))
    }
}

macro_rules! bundle {
    ($($t:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($t: Bundle),*> Bundle for ($($t,)*) {
            fn component_meta<F: FnMut(&ComponentMeta)>(components: &mut Components, ids: &mut F) {
                $(
                    $t::component_meta::<F>(components, ids);
                )*
            }
            // Inserted in the order of [`component_ids`]
            fn insert_components<F: FnMut(OwnedPtr)>(self, f: &mut F) {
                let ($($t,)*) = self;
                $(
                    $t.insert_components::<F>(f);
                )*
            }
        }
    }
}

ecs_macro::all_tuples!(bundle, 1, 10, B);

#[derive(Debug, Default)]
pub struct Bundles {
    meta: fxhash::FxHashMap<std::any::TypeId, BundleMeta>,
}

impl Bundles {
    pub fn register<B: Bundle>(
        &mut self,
        archetype: ArchId,
        table: TableId,
        component_ids: Box<[ComponentMeta]>,
    ) -> &BundleMeta {
        let id = std::any::TypeId::of::<B>();
        if let Some(_) = self.meta.get(&id) {
            warn!("Unnecessarily registering meta");
        } else {
            trace!("Registering bundle: {}", std::any::type_name::<B>());
            let meta = BundleMeta::new(archetype, table, component_ids);
            self.meta.insert(id, meta);
        }

        // just created
        self.meta::<B>().unwrap()
    }

    pub fn meta<B: Bundle>(&self) -> Option<&BundleMeta> {
        self.meta.get(&std::any::TypeId::of::<B>())
    }
}

#[derive(Debug)]
pub struct BundleMeta {
    pub arch_id: ArchId,
    pub table_id: TableId,
    // Unsorted
    pub component_ids: Box<[ComponentMeta]>,
}

impl BundleMeta {
    pub fn new(arch_id: ArchId, table_id: TableId, component_ids: Box<[ComponentMeta]>) -> Self {
        Self {
            arch_id,
            table_id,
            component_ids,
        }
    }
}
