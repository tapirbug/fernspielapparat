pub use builder::Builder as SensorsBuilder;

use crate::senses::dial::Input;
use crate::senses::{Error, Sense};
use log::error;

/// Runs senses in the background, making it possible to
/// poll them without blocking.
pub struct Sensors(Vec<Box<dyn Sense>>);

impl Sensors {
    /// Creates a builder for sensors, where background
    /// senses can be added and are evaluated in their
    /// own threads.
    pub fn builder() -> SensorsBuilder {
        SensorsBuilder::new()
    }

    /// Sensors where polled input is always `None`.
    pub fn blind() -> Self {
        Sensors(vec![])
    }

    /// Polls all sensors and exits early if input has
    /// been received.
    pub fn poll(&mut self) -> Option<Input> {
        let mut first_input = None;
        let mut removals = Vec::new();
        for (idx, sensor) in self.0.iter_mut().enumerate() {
            match sensor.poll() {
                Err(Error::Fatal(e)) => {
                    error!("Giving up on sensor after fatal error: {}", e);
                    removals.push(idx);
                }
                Err(Error::WouldBlock) => (),
                Ok(input) => {
                    first_input = Some(input);
                    break;
                }
            }
        }

        for idx in removals {
            self.0.swap_remove(idx);
        }

        first_input
    }
}

mod builder {
    use super::{Sense, Sensors};
    use crate::senses::bg::BackgroundSense;
    use crate::senses::dial::{HardwareDial, Queue, QueueInput, StdinDial};
    use crate::Phone;

    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    const POLL_INTERVAL: Duration = Duration::from_millis(150);

    pub struct Builder {
        may_block: Vec<Box<dyn Sense + Send>>,
        non_blocking: Vec<Box<dyn Sense>>,
    }

    impl Builder {
        pub fn new() -> Self {
            Builder {
                may_block: Vec::new(),
                non_blocking: Vec::new(),
            }
        }

        /// Enables background input via the given sense
        /// that may block.
        ///
        /// The sense will be invoked from a background
        /// thread that is spawned at build time.
        pub fn background(&mut self, sense: impl Sense + Send + 'static) -> &mut Self {
            self.may_block.push(Box::new(sense));
            self
        }

        fn non_blocking(&mut self, sense: impl Sense + 'static) -> &mut Self {
            self.non_blocking.push(Box::new(sense));
            self
        }

        /// Enables input from stdin. It accepts 0-9 (dial),
        /// h (hang up) and p (pick up). Newlines may be required
        /// for flushing.
        pub fn stdin(&mut self) -> &mut Self {
            self.background(StdinDial::new())
        }

        pub fn i2c_dial(&mut self, phone: &Arc<Mutex<Phone>>) -> &mut Self {
            self.background(HardwareDial::new(phone))
        }

        pub fn queue(&mut self) -> (&mut Self, QueueInput) {
            let (queue, input) = Queue::new();
            self.non_blocking(queue);
            (self, input)
        }

        pub fn build(self) -> Sensors {
            Sensors(
                self.may_block
                    .into_iter()
                    .map(|sensor| BackgroundSense::spawn(sensor, Some(POLL_INTERVAL)))
                    .chain(self.non_blocking.into_iter())
                    .collect(),
            )
        }
    }
}
