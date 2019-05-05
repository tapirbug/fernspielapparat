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
        let mut ring = Ring {
            phone: Arc::clone(phone),
            start: Instant::now(),
            duration,
            is_done: false,
        };

        ring.start()?;

        Ok(ring)
    }
}

impl Act for Ring {
    fn update(&mut self) -> Result<(), Error> {
        if !self.is_done {
            let long_enough = self.start.elapsed().gt(&self.duration);
            if long_enough {
                self.is_done = true;
                self.stop()?;
            }
        }

        Ok(())
    }

    fn done(&self) -> Result<bool, Error> {
        Ok(self.is_done)
    }

    fn cancel(&mut self) -> Result<(), Error> {
        self.stop()
    }
}

impl Ring {
    fn start(&mut self) -> Result<(), Error> {
        let mut phone = self.phone.lock().expect("Failed to obtain lock on phone");
        Ok(phone.ring()?)
    }

    fn stop(&mut self) -> Result<(), Error> {
        let mut phone = self.phone.lock().expect("Failed to obtain lock on phone");
        Ok(phone.unring()?)
    }
}
