use std::{
    fmt::Debug,
    sync::mpsc::{SendError, Sender},
};

use ecs_derive::InternalResource;

#[derive(Debug, InternalResource)]
pub struct ThreadMessageSender<T>
where
    T: 'static + Send + Sync + Debug,
{
    sender: Sender<T>,
    pub previous_message: Option<T>,
}

unsafe impl<T> Send for ThreadMessageSender<T> where T: 'static + Send + Sync + Debug {}
unsafe impl<T> Sync for ThreadMessageSender<T> where T: 'static + Send + Sync + Debug {}

impl<T> ThreadMessageSender<T>
where
    T: 'static + Send + Sync + Debug + Clone,
{
    pub fn new(sender: Sender<T>) -> Self {
        Self {
            sender,
            previous_message: None,
        }
    }

    pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.sender.send(msg)
    }

    pub fn track_and_send(&mut self, msg: T) -> Result<(), SendError<T>> {
        self.sender.send(msg.clone())?;
        self.previous_message = Some(msg);

        Ok(())
    }
}
