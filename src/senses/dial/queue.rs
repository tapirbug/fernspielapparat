use crate::senses::{Error, Input, Sense};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use failure::format_err;

pub type QueueInput = Sender<Input>;

/// FIFO structure where inputs from different sources, e.g.
/// remote control can come in in bulk and are then emitted
/// per timestep.
pub struct Queue(Receiver<Input>);

impl Queue {
    pub fn new() -> (Self, QueueInput) {
        let (tx, rx) = unbounded();
        (Queue(rx), tx)
    }
}

impl Sense for Queue {
    /// Tries to get the next input from stdin, if any.
    fn poll(&mut self) -> Result<Input, Error> {
        self.0.try_recv().map_err(|e| match e {
            TryRecvError::Empty => Error::WouldBlock,
            TryRecvError::Disconnected => {
                Error::Fatal(format_err!("Remote end disconnected from queue sense"))
            }
        })
    }
}
