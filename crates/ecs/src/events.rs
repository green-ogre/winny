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
    fn index(&self) -> usize {
        self.id()
    }
}

#[derive(Debug, InternalResource)]
pub struct Events<E: Event> {
    storage: Vec<E>,
    _phantom: PhantomData<E>,
}

unsafe impl<E: Event> Sync for Events<E> {}
unsafe impl<E: Event> Send for Events<E> {}

impl<E: Event> Events<E> {
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn is_empty(&self) -> bool {
        self.storage.len() == 0
    }

    pub fn flush(&mut self) {
        self.storage.drain(..);
    }

    pub fn push(&mut self, val: E) {
        self.storage.push(val);
    }

    pub fn peak(&self) -> Option<&E> {
        (self.storage.is_empty()).then_some(self.storage.last().unwrap())
    }

    pub fn append(&mut self, vals: impl Iterator<Item = E>) {
        self.storage.extend(vals);
    }

    pub fn peak_read(&self) -> impl Iterator<Item = &E> {
        self.storage.iter()
    }

    pub fn read(&mut self) -> impl Iterator<Item = E> {
        let mut new = Vec::with_capacity(self.storage.len());
        std::mem::swap(&mut new, &mut self.storage);
        new.into_iter()
    }
}

#[derive(Debug)]
pub struct EventWriter<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>, resource_id: ResourceId) -> Self {
        Self {
            events: unsafe { world.get_resource_mut_ref_by_id(resource_id) },
        }
    }

    pub fn send(&mut self, event: E) {
        self.events.push(event);
    }
}

#[derive(Debug)]
pub struct EventReader<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventReader<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>, resource_id: ResourceId) -> Self {
        Self {
            events: unsafe { world.get_resource_mut_ref_by_id(resource_id) },
        }
    }

    pub fn peak(&self) -> Option<&E> {
        self.events.peak()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.len() == 0
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
