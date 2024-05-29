use self::unsafe_world::UnsafeWorldCell;
use logger::error;
use std::marker::PhantomData;

use super::*;

pub trait Event: TypeGetter + Send + Sync {}

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

#[derive(Debug)]
pub struct Events {
    events: SparseSet<EventId, EventQueue>,
}

unsafe impl Sync for Events {}
unsafe impl Send for Events {}

impl Events {
    pub fn new() -> Self {
        Self {
            events: SparseSet::new(),
        }
    }

    pub fn insert<E: Event>(&mut self, event_id: EventId) {
        self.events.insert(event_id, EventQueue::new::<E>());
    }

    pub fn get(&self, event_id: EventId) -> Option<&EventQueue> {
        self.events.get(&event_id)
    }

    pub fn get_mut(&mut self, event_id: EventId) -> Option<&mut EventQueue> {
        self.events.get_mut(&event_id)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut EventQueue> {
        self.events.values_mut().iter_mut()
    }

    pub fn new_id(&self) -> EventId {
        EventId::new(self.events.len())
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

pub struct EventWriter<'w, T> {
    world: UnsafeWorldCell<'w>,
    event_id: EventId,
    event_type: PhantomData<T>,
}

impl<'a, E: Event> std::fmt::Debug for EventWriter<'a, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventWriter")
            .field("{:#?}", unsafe {
                self.world
                    .read_only()
                    .events
                    .get(self.event_id)
                    .expect("Event type is registered")
            })
            .finish()
    }
}

impl<'w, E: Event> EventWriter<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        return Self {
            world,
            event_id: unsafe { world.read_only() }.get_event_id::<E>(),
            event_type: PhantomData,
        };
    }

    pub fn send(&mut self, event: E) {
        if let Some(queue) = unsafe { self.world.read_and_write().events.get_mut(self.event_id) } {
            let _ = queue.push(event);
        } else {
            error!("Event not registered: {}", E::type_name().as_string());
            panic!();
        }
    }
}

pub struct EventReader<'w, T> {
    world: UnsafeWorldCell<'w>,
    event_id: EventId,
    event_type: PhantomData<T>,
}

impl<'a, E: Event> std::fmt::Debug for EventReader<'a, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventReader")
            .field("{:#?}", unsafe {
                self.world
                    .read_only()
                    .events
                    .get(self.event_id)
                    .expect("Event is registered")
            })
            .finish()
    }
}

impl<'w, E: Event + TypeGetter> EventReader<'w, E> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        return Self {
            world,
            event_id: unsafe { world.read_only() }.get_event_id::<E>(),
            event_type: PhantomData,
        };
    }

    pub fn read(self) -> impl Iterator<Item = E> {
        if let Some(queue) = unsafe { self.world.read_and_write().events.get_mut(self.event_id) } {
            queue.read()
        } else {
            error!("Event not registered: {}", E::type_name().as_string());
            panic!();
        }
    }
}
