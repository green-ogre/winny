use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, SendError, Sender, TryIter},
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

#[derive(Debug, InternalResource)]
pub struct ChannelReciever<T>
where
    T: 'static + Send + Sync + Debug,
{
    reciever: Receiver<T>,
}

unsafe impl<T> Send for ChannelReciever<T> where T: 'static + Send + Sync + Debug {}
unsafe impl<T> Sync for ChannelReciever<T> where T: 'static + Send + Sync + Debug {}

impl<T> ChannelReciever<T>
where
    T: 'static + Send + Sync + Debug,
{
    pub fn new(reciever: Receiver<T>) -> Self {
        Self { reciever }
    }

    pub fn try_recv(&self) -> Result<T, ()> {
        self.reciever.try_recv().map_err(|_| ())
    }

    pub fn try_iter(&self) -> TryIter<'_, T> {
        self.reciever.try_iter()
    }
}
