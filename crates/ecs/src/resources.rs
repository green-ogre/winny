use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
};

use crate::{any::*, World};

pub trait Resource {}

pub trait ResourceStorage: std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T: std::fmt::Debug + TypeGetter> ResourceStorage for RefCell<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

pub struct Res<'a, T> {
    value: Ref<'a, T>,
}

impl<'a, T: Resource + TypeGetter> Res<'a, T> {
    pub fn new(world: &'a World) -> Self {
        Self {
            value: world.resource::<T>(),
        }
    }
}

pub struct ResMut<'a, T> {
    value: RefMut<'a, T>,
}

impl<'a, T: Resource + TypeGetter> ResMut<'a, T> {
    pub fn new(world: &'a World) -> Self {
        world.resource_mut::<T>();
        Self {
            value: world.resource_mut::<T>(),
        }
    }
}

impl<'a, T: Resource + TypeGetter> AsRef<T> for Res<'a, T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<'a, T: Resource + TypeGetter> AsMut<T> for ResMut<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
