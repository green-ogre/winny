use std::{
    cell::{RefCell, RefMut},
    collections::VecDeque,
    fmt::{Debug, Write},
    marker::PhantomData,
};

use crate::{
    any::*, new_dumb_drop, unsafe_world::UnsafeWorldCell, world, DumbVec, MutableSparseSet, World,
};

pub trait Event: Send + Sync {}

#[derive(Debug)]
pub struct Events {
    events: MutableSparseSet<TypeId, EventQueue>,
}

unsafe impl Sync for Events {}
unsafe impl Send for Events {}

impl Events {
    pub fn new() -> Self {
        Self {
            events: MutableSparseSet::new(),
        }
    }

    pub fn insert<E: Event + TypeGetter>(&mut self) {
        self.events.insert(E::type_id(), EventQueue::new::<E>());
    }

    pub fn queue<E: Event + TypeGetter>(&self) -> Option<&EventQueue> {
        self.events.get_value(&E::type_id())
    }

    pub fn queue_mut<E: Event + TypeGetter>(&mut self) -> Option<&mut EventQueue> {
        self.events.get_value_mut(&E::type_id())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut EventQueue> {
        self.events.iter_mut()
    }
}

#[derive(Debug)]
pub struct EventQueue {
    storage: DumbVec,
}

impl EventQueue {
    pub fn new<E: Event + TypeGetter>() -> Self {
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

    pub fn push<E>(&mut self, val: E) -> Result<(), ()> {
        self.storage.push(val)
    }

    pub fn read<E>(&mut self) -> impl Iterator<Item = E> {
        self.storage.into_vec::<E>().into_iter()
    }
}

pub struct EventWriter<'w, T> {
    world: UnsafeWorldCell<'w>,
    event_type: PhantomData<T>,
}

impl<'a, E: Event + TypeGetter> std::fmt::Debug for EventWriter<'a, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventWriter")
            .field("{:#?}", unsafe {
                self.world
                    .read_and_write()
                    .events
                    .queue::<E>()
                    .expect("Event type is registered")
            })
            .finish()
    }
}

impl<'w, E: Event + TypeGetter> EventWriter<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        return Self {
            world,
            event_type: PhantomData,
        };
    }

    pub fn send(&mut self, event: E) {
        if let Some(queue) = unsafe { self.world.read_and_write().events.queue_mut::<E>() } {
            queue.push(event);
        } else {
            panic!("Event not registered: {}", E::type_name().as_string());
        }
    }
}

pub struct EventReader<'w, T> {
    world: UnsafeWorldCell<'w>,
    event_type: PhantomData<T>,
}

impl<'a, E: Event + TypeGetter> std::fmt::Debug for EventReader<'a, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventReader")
            .field("{:#?}", unsafe {
                self.world
                    .read_and_write()
                    .events
                    .queue::<E>()
                    .expect("Event type is registered")
            })
            .finish()
    }
}

impl<'w, E: Event + TypeGetter> EventReader<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        return Self {
            world,
            event_type: PhantomData,
        };
    }

    pub fn read(self) -> impl Iterator<Item = E> {
        if let Some(queue) = unsafe { self.world.read_and_write().events.queue_mut::<E>() } {
            queue.read()
        } else {
            panic!("Event not registered: {}", E::type_name().as_string());
        }
    }
}
