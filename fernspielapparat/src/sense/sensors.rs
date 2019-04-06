use crate::sense::dial::Input;
use crate::sense::{Error, Sense};
use builder::Builder;
use log::error;

/// Runs senses in the background, making it possible to
/// poll them without blocking.
pub struct Sensors(Vec<Box<dyn Sense>>);

impl Sensors {
    pub fn builder() -> Builder {
        Builder::new()
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
    use crate::sense::bg::BackgroundSense;
    use std::time::Duration;

    pub struct Builder {
        may_block: Vec<Box<dyn Sense + Send>>,
    }

    impl Builder {
        pub fn new() -> Self {
            Builder {
                may_block: Vec::new(),
            }
        }

        /// Enables background input via the given sense
        /// that may block.
        ///
        /// The sense will be invoked from a background
        /// thread that is spawned at build time.
        pub fn background(mut self, sense: impl Sense + Send + 'static) -> Self {
            self.may_block.push(Box::new(sense));
            self
        }

        pub fn build(self) -> Sensors {
            Sensors(
                self.may_block
                    .into_iter()
                    .map(|sensor| BackgroundSense::spawn(sensor, Some(Duration::from_millis(50)))) // No more than 20 inputs/second
                    .collect(),
            )
        }
    }
}
