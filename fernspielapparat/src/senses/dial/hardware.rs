use crate::phone::Phone;
use crate::senses::{dial::Input, Error, Sense};
use log::warn;
use std::io;
use std::sync::{Arc, Mutex};

pub struct HardwareDial {
    phone: Arc<Mutex<Phone>>,
    last_input: Option<Input>,
}

impl HardwareDial {
    pub fn new(phone: &Arc<Mutex<Phone>>) -> Self {
        HardwareDial {
            phone: Arc::clone(phone),
            last_input: None,
        }
    }

    /// Looks at the error and downgrades it to WouldBlock
    /// if expects that the error will go away in the future.
    fn evaluate_error(&self, error: io::Error) -> Error {
        if cfg!(unix) && error.raw_os_error() == Some(121) {
            warn!("Non-critical I/O fail 121 (probably recovers by itself)");
            return Error::WouldBlock;
        }

        match error.kind() {
            io::ErrorKind::WouldBlock => Error::WouldBlock,
            _ => Error::fatal(error),
        }
    }

    /// The phone does not block on input but repeatedly sends
    /// hangups and pickups if no input was done.
    ///
    /// Consolidate these duplicate inputs.
    fn combine_with_old(&mut self, new_input: Input) -> Result<Input, Error> {
        let combined = match (self.last_input, new_input) {
            (Some(Input::PickUp), Input::PickUp) => Err(Error::WouldBlock),
            (Some(Input::HangUp), Input::HangUp) => Err(Error::WouldBlock),
            _ => Ok(new_input),
        };

        self.last_input = Some(new_input);

        combined
    }
}

impl Sense for HardwareDial {
    fn poll(&mut self) -> Result<Input, Error> {
        let input = self
            .phone
            .lock()
            .expect("Failed to obtain lock on phone")
            .poll();

        input
            .map_err(|e| self.evaluate_error(e))
            .and_then(|i| self.combine_with_old(i))
    }
}
