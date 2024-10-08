use std::{any::TypeId, ptr::NonNull};
use util::tracing::trace;
use super::*;

// https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/system/system_param.rs#L483
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Resource`",
    label = "invalid `Resource`",
    note = "consider annotating `{Self}` with `#[derive(Resource)]`"
)]
#[cfg(not(target_arch = "wasm32"))]
pub trait Resource: Send + Sync + 'static {}
#[cfg(target_arch = "wasm32")]
pub trait Resource: 'static {}

#[derive(Debug)]
pub struct Res<'a, R> {
    value: &'a R,
}

impl<R> Deref for Res<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, R: Resource> Res<'w, R> {
    pub fn new(value: &'w R) -> Self {
        Self { value }
    }
}

#[derive(Debug)]
pub struct ResMut<'a, R> {
    value: &'a mut R,
}

impl<R> Deref for ResMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<R> DerefMut for ResMut<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<'w, R: Resource> ResMut<'w, R> {
    pub fn new(value: &'w mut R) -> Self {
        Self { value }
    }
}

impl<R: Resource> AsRef<R> for Res<'_, R> {
    fn as_ref(&self) -> &R {
        self.value
    }
}

impl<R: Resource> AsMut<R> for ResMut<'_, R> {
    fn as_mut(&mut self) -> &mut R {
        self.value
    }
}

pub struct Take<R: Resource> {
    value: R
}

impl<R: Resource> Deref for Take<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<R: Resource> DerefMut for Take<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<R: Resource> Take<R> {
    pub fn new(value: R) -> Self {
        Self {
            value,
        }
    }

    pub fn into_inner(self) -> R {
        self.value
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(usize);

impl ResourceId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl SparseArrayIndex for ResourceId {
    fn index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug)]
pub struct Resources {
    pub resources: SparseArray<ResourceId, DumbVec>,
    pub resource_id_table: fxhash::FxHashMap<ResourceId, ResourceMeta>,
    pub id_table: fxhash::FxHashMap<std::any::TypeId, ResourceMeta>,
    pub next_id: usize,
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            resources: SparseArray::new(),
            id_table: fxhash::FxHashMap::default(),
            resource_id_table: fxhash::FxHashMap::default(),
            next_id: 0,
        }
    }
}

impl Resources {
    pub fn register<R: Resource>(&mut self) -> ResourceId {
        let type_id = std::any::TypeId::of::<R>();
        if self.id_table.contains_key(&type_id) {
            self.id_table.get(&type_id).unwrap().resource_id
        } else {
            trace!("Registering resource: {}", std::any::type_name::<R>(),);

            let id = ResourceId::new(self.next_id);
            self.next_id += 1;

            let meta = ResourceMeta {
                resource_id: id,
                type_id,
                name: std::any::type_name::<R>()
            };

            self.id_table.insert(type_id, meta.clone());
            self.resource_id_table.insert(meta.resource_id, meta.clone());

            meta.resource_id
    }
    }

    pub fn insert<R: Resource>(&mut self, res: R, id: ResourceId) {
        if let Some(storage) = self.resources.get_mut(&id) {
            // caller promises that R and ResourceId match
            unsafe {
                if !storage.is_empty() {
                    storage.replace_drop::<R>(res, 0);
                } else {
                    storage.push::<R>(res);
                }
            }
        } else {
            let mut storage = DumbVec::with_capacity::<R>(1);
            // storage newly created with type
            unsafe { storage.push(res) };

            self.resources.insert(id.index(), storage);
        }
    }

    pub fn take<R: Resource> (&mut self, id: ResourceId) -> Option<R> {
        if let Some(res) = self.resources.get_mut(&id) {
            Self::is_valid(res).then(|| unsafe {res.pop::<R>()})
        } else {
            panic!("tried to take resource that does not exist");
        }
    }

    pub fn id<R: Resource>(&self) -> Option<ResourceId> {
        let id = std::any::TypeId::of::<R>();
        self.id_table.get(&id).map(|m| m.resource_id)
    }

    pub fn id_unwrapped<R: Resource>(&self) -> ResourceId {
        if let Some(id) = self.id::<R>() {
            id
        } else {
            util::tracing::error!("Resource [`{}`] is not registered. Remember to `world.insert_resource(..)` \
                or `world.register_resource::<R>` if the resource is uninitialized.", std::any::type_name::<R>());
            panic!();
        }
    }

    pub fn get_id(&self, type_id: &TypeId) -> Option<ResourceId> {
        self.id_table.get(&type_id).map(|m| m.resource_id)
    }

    pub fn meta(&self, type_id: &TypeId) -> Option<&ResourceMeta> {
        self.id_table.get(type_id)
    }

    pub fn get_ptr<R: Resource>(&self, id: ResourceId) -> Option<NonNull<R>> {
        self.resources
            .get(&id)
            .map(|res| Self::is_valid(res).then(|| 
                // caller promises that R and ResourceId match
                unsafe { res.get_unchecked(0).cast::<R>() }
            ))?
    }

    pub unsafe fn get_raw_ptr(&self, id: ResourceId) -> Option<NonNull<u8>> {
        self.resources
            .get(&id)
            .map(|res| Self::is_valid(res).then(|| 
                // caller promises that R and ResourceId match
                unsafe { res.get_unchecked(0) }
            ))?
    }

    fn is_valid(res: &DumbVec) -> bool {
        res.len() == 1
    }
}

#[derive(Debug, Clone)]
pub struct ResourceMeta {
    pub resource_id: ResourceId,
    pub type_id: TypeId,
    pub name: &'static str,
}
