//! Typed channels between Hydra modules.
//! Every channel carries Signals — nothing else.

use crate::{constants::BUS_BUFFER_CAPACITY, errors::AnimusError, semiring::signal::Signal};
use tokio::sync::mpsc;

/// A named, typed channel for sending Signals between modules.
pub struct SignalChannel {
    /// Channel name.
    pub name: String,
    sender: mpsc::Sender<Signal>,
}

impl SignalChannel {
    /// Create a new channel with the default buffer capacity.
    pub fn new(name: impl Into<String>) -> (Self, mpsc::Receiver<Signal>) {
        let (tx, rx) = mpsc::channel(BUS_BUFFER_CAPACITY);
        let channel = Self {
            name: name.into(),
            sender: tx,
        };
        (channel, rx)
    }

    /// Send a signal. Returns error if the channel is full.
    pub async fn send(&self, signal: Signal) -> Result<(), AnimusError> {
        self.sender
            .try_send(signal)
            .map_err(|_| AnimusError::BusChannelFull {
                capacity: BUS_BUFFER_CAPACITY,
                channel: self.name.clone(),
            })
    }

    /// Returns true if the channel has capacity.
    pub fn has_capacity(&self) -> bool {
        self.sender.capacity() > 0
    }
}
