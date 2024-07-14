use std::ptr::NonNull;

use any_vec::AnyVec;
use util::tracing::trace;

use super::*;

// https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/system/system_param.rs#L483
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Resource`",
    label = "invalid `Resource`",
    note = "consider annotating `{Self}` with `#[derive(Resource)]`"
)]
pub trait Resource: Send + Sync + 'static {}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    resources: SparseSet<ResourceId, AnyVec<dyn Send + Sync>>,
    next_id: usize,
    id_table: fxhash::FxHashMap<std::any::TypeId, ResourceId>,
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            resources: SparseSet::new(),
            next_id: 0,
            id_table: fxhash::FxHashMap::default(),
        }
    }
}

impl Resources {
    pub fn register<R: Resource>(&mut self) -> ResourceId {
        let type_id = std::any::TypeId::of::<R>();
        if let Some(id) = self.id_table.get(&type_id) {
            *id
        } else {
            let id = self.new_id();
            self.id_table.insert(type_id, id);

            trace!(
                "Registering resource: {} => {:?}",
                std::any::type_name::<R>(),
                id
            );

            id
        }
    }

    pub fn insert<R: Resource>(&mut self, res: R, id: ResourceId) {
        if let Some(storage) = self.resources.get_mut(&id) {
            storage.clear();
            let mut vec = unsafe { storage.downcast_mut_unchecked::<R>() };
            vec.push(res);
        } else {
            let mut storage: AnyVec<dyn Send + Sync> = AnyVec::with_capacity::<R>(1);
            {
                let mut vec = unsafe { storage.downcast_mut_unchecked::<R>() };
                vec.push(res);
            }

            self.resources.insert(id, storage);
        }
    }

    pub fn id<R: Resource>(&self) -> Option<ResourceId> {
        let id = std::any::TypeId::of::<R>();
        self.id_table.get(&id).cloned()
    }

    pub fn get_ptr<R: Resource>(&self, id: ResourceId) -> Option<NonNull<R>> {
        self.resources.get(&id).map(|res| {
            (res.len() == 1).then(|| unsafe {
                let ptr = res.downcast_ref_unchecked::<R>().as_ptr();
                NonNull::new(ptr as *mut R)
            })?
        })?
    }

    fn new_id(&mut self) -> ResourceId {
        let id = self.next_id;
        self.next_id += 1;

        ResourceId(id)
    }
}
