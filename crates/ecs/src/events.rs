use self::unsafe_world::UnsafeWorldCell;
use logger::error;
use std::marker::PhantomData;

use super::*;

pub trait Event: Send + Sync + 'static {}

#[derive(Debug, Clone, Copy)]
pub struct EventId(usize);

impl EventId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn id(&self) -> usize {
        self.0
    }
}

impl SparseArrayIndex for EventId {
    fn to_index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug, InternalResource)]
pub struct Events<E: Event> {
    storage: DumbVec,
    _phantom: PhantomData<E>,
}

unsafe impl<E: Event> Sync for Events<E> {}
unsafe impl<E: Event> Send for Events<E> {}

impl<E: Event> Events<E> {
    pub fn new() -> Self {
        Self {
            storage: DumbVec::new(std::alloc::Layout::new::<E>(), 0, new_dumb_drop::<E>()),
            _phantom: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn flush(&mut self) {
        self.storage.clear_drop();
    }

    pub fn push(&mut self, val: E) -> Result<(), IntoStorageError> {
        self.storage.push(val)
    }

    pub fn read(&mut self) -> impl Iterator<Item = E> {
        self.storage.into_vec::<E>().into_iter()
    }
}

#[derive(Debug)]
pub struct EventQueue {
    storage: DumbVec,
}

impl EventQueue {
    pub fn new<E: Event>() -> Self {
        Self {
            storage: DumbVec::new(std::alloc::Layout::new::<E>(), 0, new_dumb_drop::<E>()),
        }
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn flush(&mut self) {
        self.storage.clear_drop();
    }

    pub fn push<E>(&mut self, val: E) -> Result<(), IntoStorageError> {
        self.storage.push(val)
    }

    pub fn read<E>(&mut self) -> impl Iterator<Item = E> {
        self.storage.into_vec::<E>().into_iter()
    }
}

#[derive(Debug)]
pub struct EventWriter<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>, resource_id: ResourceId) -> Self {
        Self {
            events: ResMut::new(world, resource_id),
        }
    }

    pub fn send(&mut self, event: E) {
        let _ = self.events.push(event);
    }
}

#[derive(Debug)]
pub struct EventReader<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventReader<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>, resource_id: ResourceId) -> Self {
        Self {
            events: ResMut::new(world, resource_id),
        }
    }

    pub fn read(mut self) -> impl Iterator<Item = E> {
        self.events.read()
    }
}
