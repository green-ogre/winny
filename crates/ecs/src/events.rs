use std::{
    cell::{RefCell, RefMut},
    collections::VecDeque,
    fmt::{Debug, Write},
    marker::PhantomData,
};

use crate::{any::*, World};

pub trait Event: Send {}

pub trait EventQueue: Send + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn flush(&mut self);
}

impl<T: Send + TypeGetter + Debug> EventQueue for RefCell<VecDeque<T>> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn flush(&mut self) {
        let _ = self.replace(VecDeque::new());
    }
}

pub trait RecieveWorldEvents {
    type Output;

    fn read(self) -> Self::Output;
}

pub trait SendWorldEvents {
    fn send<U: Event + TypeGetter>(&mut self, event: U);
}

pub struct EventWriter<'a, T> {
    world: &'a World,
    event_type: PhantomData<T>,
}

impl<'a, T: Event + TypeGetter> std::fmt::Debug for EventWriter<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("EventWriter: {}", T::type_name().as_string()))
    }
}

impl<'a, T> EventWriter<'a, T> {
    pub fn new(world: &'a World) -> Self {
        return Self {
            world,
            event_type: PhantomData,
        };
    }
}

impl<'b, T: Event + TypeGetter> SendWorldEvents for EventWriter<'b, T> {
    fn send<U: Event + TypeGetter>(&mut self, event: U) {
        for event_vec in self.world.events.iter() {
            if let Some(event_vec) = event_vec.as_any().downcast_ref::<RefCell<VecDeque<U>>>() {
                event_vec.borrow_mut().push_back(event);
                return;
            }
        }

        panic!(
            "Event not registered: {}",
            std::any::type_name::<dyn Event>()
        );
    }
}

pub struct EventReader<'a, T> {
    world: &'a World,
    event_type: PhantomData<T>,
}

impl<'a, T: Event + TypeGetter> std::fmt::Debug for EventReader<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("EventReader: {}", T::type_name().as_string()))
    }
}

impl<'a, T> EventReader<'a, T> {
    pub fn new(world: &'a World) -> Self {
        return Self {
            world,
            event_type: PhantomData,
        };
    }
}

impl<'b, T: Event + TypeGetter> RecieveWorldEvents for EventReader<'b, T> {
    type Output = RefMut<'b, VecDeque<T>>;

    fn read(self) -> Self::Output {
        for event_vec in self.world.events.iter() {
            if let Some(event_vec) = event_vec.as_any().downcast_ref::<RefCell<VecDeque<T>>>() {
                // *self.drain.borrow_mut() = event_vec.borrow_mut().drain(..);
                return event_vec.borrow_mut();
            }
        }

        panic!("Event not registered: {}", std::any::type_name::<T>());
    }
}
