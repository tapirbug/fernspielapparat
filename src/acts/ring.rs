use crate::acts::Act;
use crate::phone::Phone;
use failure::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Ring {
    phone: Arc<Mutex<Phone>>,
    start: Instant,
    duration: Duration,
    is_done: bool,
}

impl Ring {
    pub fn new(phone: &Arc<Mutex<Phone>>, duration: Duration) -> Result<Self, Error> {
        let ring = Ring {
            phone: Arc::clone(phone),
            start: Instant::now(),
            duration,
            is_done: false,
        };

        Ok(ring)
    }
}

impl Act for Ring {
    fn activate(&mut self) -> Result<(), Error> {
        let mut phone = self.phone.lock().expect("Failed to obtain lock on phone");
        Ok(phone.ring()?)
    }

    fn update(&mut self) -> Result<(), Error> {
        if !self.is_done {
            let long_enough = self.start.elapsed().gt(&self.duration);
            if long_enough {
                self.cancel()?;
            }
        }

        Ok(())
    }

    fn done(&self) -> Result<bool, Error> {
        Ok(self.is_done)
    }

    fn cancel(&mut self) -> Result<(), Error> {
        let mut phone = self.phone.lock().expect("Failed to obtain lock on phone");
        phone.unring()?;
        self.is_done = true;
        Ok(())
    }
}
