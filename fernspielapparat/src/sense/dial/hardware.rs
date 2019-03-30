use crate::phone::Phone;
use crate::sense::{dial::Input, Error, Sense};
use std::sync::{Arc, Mutex};

pub struct HardwareDial(Arc<Mutex<Phone>>);

impl HardwareDial {
    pub fn new(phone: &Arc<Mutex<Phone>>) -> Self {
        HardwareDial(Arc::clone(phone))
    }
}

impl Sense for HardwareDial {
    fn poll(&mut self) -> Result<Input, Error> {
        let input = self
            .0
            .lock()
            .expect("Failed to obtain lock on phone")
            .poll();

        // TODO detect fatal errors
        input.map_err(|_| Error::WouldBlock)
    }
}
