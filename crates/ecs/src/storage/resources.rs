use logger::error;

use crate::unsafe_world::UnsafeWorldCell;

use super::*;

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

impl<R: Resource> Res<'_, R> {
    pub fn new<'w>(world: UnsafeWorldCell<'w>, id: ResourceId) -> Self {
        Self {
            value: unsafe { &*world.resource_ptr(id) },
        }
    }

    pub fn try_new<'w>(world: UnsafeWorldCell<'w>, id: ResourceId) -> Option<Self> {
        unsafe {
            if let Some(ptr) = world.read_only().resources.try_resource_by_id(id) {
                Some(Self { value: &*ptr })
            } else {
                None
            }
        }
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

impl<R: Resource> ResMut<'_, R> {
    pub fn new<'w>(world: UnsafeWorldCell<'w>, id: ResourceId) -> Self {
        Self {
            value: unsafe { &mut *world.resource_ptr_mut(id) },
        }
    }

    pub fn try_new<'w>(world: UnsafeWorldCell<'w>, id: ResourceId) -> Option<Self> {
        unsafe {
            if let Some(ptr) = world.read_and_write().resources.try_resource_mut_by_id(id) {
                Some(Self { value: &mut *ptr })
            } else {
                None
            }
        }
    }
}

impl<R: Resource> AsRef<R> for Res<'_, R> {
    fn as_ref(&self) -> &R {
        &self.value
    }
}

impl<R: Resource> AsMut<R> for ResMut<'_, R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.value
    }
}

#[derive(Debug, Clone, Copy)]
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
    fn to_index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug)]
pub struct Resources {
    resources: SparseSet<ResourceId, DumbVec>,
}

unsafe impl Sync for Resources {}
unsafe impl Send for Resources {}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: SparseSet::new(),
        }
    }

    pub fn insert<R: Resource>(&mut self, res: R, id: ResourceId) {
        let mut storage = DumbVec::new(std::alloc::Layout::new::<R>(), 1, new_dumb_drop::<R>());
        storage.push(res).unwrap();

        self.resources.insert(id, storage);
    }

    pub fn insert_storage(&mut self, storage: DumbVec, id: ResourceId) {
        self.resources.insert(id, storage);
    }

    pub unsafe fn get_resource_by_id<R: Resource>(&self, id: ResourceId) -> &R {
        if let Some(res) = self.resources.get(&id) {
            return res.get_unchecked(0).cast::<R>().as_ref();
        } else {
            error!(
                "Resource [{}] does not exist: Remeber to 'app.insert_resource::<...>()' your resource!",
                std::any::type_name::<R>()
            );
            panic!();
        }
    }

    pub fn get_resource_mut_by_id<R: Resource>(&mut self, id: ResourceId) -> &mut R {
        if let Some(res) = self.resources.get_mut(&id) {
            return unsafe { res.get_unchecked(0).cast::<R>().as_mut() };
        } else {
            error!(
            "Resource [{}] does not exist: Remeber to 'app.insert_resource::<...>()' your resource!",
                std::any::type_name::<R>()
        );
            panic!();
        }
    }

    pub unsafe fn try_resource_by_id<R: Resource>(&self, id: ResourceId) -> Option<&R> {
        if let Some(res) = self.resources.get(&id) {
            Some(res.get_unchecked(0).cast::<R>().as_ref())
        } else {
            None
        }
    }

    pub unsafe fn try_resource_mut_by_id<R: Resource>(&self, id: ResourceId) -> Option<&mut R> {
        if let Some(res) = self.resources.get(&id) {
            Some(res.get_mut_unchecked(0).cast::<R>().as_mut())
        } else {
            None
        }
    }

    pub fn new_id(&self) -> ResourceId {
        ResourceId::new(self.resources.len())
    }

    pub fn contains(&self, id: ResourceId) -> bool {
        self.resources.get(&id).is_some()
    }
}
