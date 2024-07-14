use std::{
    fmt::Debug,
    ops::Deref,
    sync::mpsc::{Receiver, SendError, Sender},
};

use ecs_macro::InternalResource;

#[derive(Debug, InternalResource)]
pub struct ChannelSender<T>
where
    T: 'static + Send + Sync + Debug,
{
    sender: Sender<T>,
}

unsafe impl<T> Send for ChannelSender<T> where T: 'static + Send + Sync + Debug {}
unsafe impl<T> Sync for ChannelSender<T> where T: 'static + Send + Sync + Debug {}

impl<T> ChannelSender<T>
where
    T: 'static + Send + Sync + Debug + Clone,
{
    pub fn new(sender: Sender<T>) -> Self {
        Self { sender }
    }

    pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.sender.send(msg)
    }
}

#[derive(InternalResource)]
pub struct ChannelReciever<T>
where
    T: 'static + Send + Sync,
{
    reciever: Receiver<T>,
}

unsafe impl<T> Send for ChannelReciever<T> where T: 'static + Send + Sync {}
unsafe impl<T> Sync for ChannelReciever<T> where T: 'static + Send + Sync {}

impl<T> ChannelReciever<T>
where
    T: 'static + Send + Sync,
{
    pub fn new(reciever: Receiver<T>) -> Self {
        Self { reciever }
    }
}

impl<T: 'static + Send + Sync> Deref for ChannelReciever<T> {
    type Target = Receiver<T>;
    fn deref(&self) -> &Self::Target {
        &self.reciever
    }
}
