use crate::sense::dial::Input;
use crate::sense::{Sense, Error};
use builder::Builder;

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
                    eprintln!("Fatal error on sensor: {}", e);
                    removals.push(idx);
                },
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
    use super::{Sensors, Sense, Error, Input};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread;

    pub struct Builder {
        may_block: Vec<Box<dyn Sense + Send>>
    }

    impl Builder {
        pub fn new() -> Self {
            Builder {
                may_block: Vec::new()
            }
        }

        /// Enables background input via the given sense
        /// that may block.
        /// 
        /// The sense will be invoked from a background
        /// thread that is spawned at build time.
        pub fn background(mut self, sense: impl Sense + Send + 'static) -> Self
        {
            self.may_block.push(Box::new(sense));
            self
        }

        pub fn build(self) -> Sensors {
            Sensors(
                self.may_block.into_iter()
                    .map(BackgroundSense::spawn)
                    .collect()
            )
        }
    }

    struct BackgroundSense(Receiver<Input>);

    impl Sense for BackgroundSense {
        fn poll(&mut self) -> Result<Input, Error> {
            self.0.try_recv()
                .or_else(|_| Err(Error::WouldBlock))
        }
    }

    impl BackgroundSense {
        fn spawn(sense: Box<dyn Sense + Send>) -> Box<dyn Sense> {
            let (tx, rx) = channel();
            thread::spawn(move || {
                keep_polling(sense, tx);
            });
            Box::new(BackgroundSense(rx))
        }
    }

    fn keep_polling(mut sense: Box<dyn Sense>, sender: Sender<Input>) {
        loop {
            match sense.poll() {
                Ok(input) => sender.send(input).expect("Could not send input back"),
                Err(Error::Fatal(e)) => break,
                Err(Error::WouldBlock) => thread::yield_now()
            }
        }
    }
}
