use self::unsafe_world::UnsafeWorldCell;
use std::{fmt::Debug, marker::PhantomData};

use super::*;

pub trait Event: Send + Sync + 'static + Debug {}

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

    pub fn push(&mut self, val: E) {
        self.storage.push(val).unwrap()
    }

    pub fn peak(&self) -> Option<&E> {
        if self.storage.len() == 0 {
            None
        } else {
            Some(unsafe { self.storage.get_unchecked(0).cast::<E>().as_ref() })
        }
    }

    pub fn append(&mut self, vals: impl Iterator<Item = E>) {
        for val in vals {
            self.storage.push(val).unwrap()
        }
    }

    pub fn peak_read(&self) -> impl Iterator<Item = &E> {
        self.storage.as_slice::<E>().iter()
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

    pub fn peak(&self) -> Option<&E> {
        self.events.peak()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn peak_read(&self) -> impl Iterator<Item = &E> {
        self.events.peak_read()
    }

    pub fn read(mut self) -> impl Iterator<Item = E> {
        self.events.read()
    }

    pub fn flush(mut self) {
        self.events.flush();
    }
}
