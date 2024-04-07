use std::{cell::UnsafeCell, marker::PhantomData};

use crate::World;

pub struct UnsafeWorldCell<'w>(UnsafeCell<World>, PhantomData<&'w World>);

impl<'w> UnsafeWorldCell<'w> {
    pub fn new(world: &World) -> Self {
        Self(UnsafeCell::new(world), PhantomData)
    }

    pub unsafe fn read_only(&self) -> &World {
        self.0.get() as &World
    }

    pub unsafe fn as_mut(&self) -> &mut World {
        self.0.get_mut()
    }
}
