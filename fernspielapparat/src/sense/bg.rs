use crate::sense::{dial::Input, Error, Sense};
use crossbeam_channel::{bounded, Receiver, Sender};
use log::debug;
use std::thread;
use std::time::Duration;

pub struct BackgroundSense(Receiver<Result<Input, Error>>);

impl Sense for BackgroundSense {
    fn poll(&mut self) -> Result<Input, Error> {
        self.0.try_recv().unwrap_or(Err(Error::WouldBlock))
    }
}

impl BackgroundSense {
    pub fn spawn(sense: Box<dyn Sense + Send>, poll_interval: Option<Duration>) -> Box<dyn Sense> {
        // 0: Block when four unconsumed inputs in the queue
        let (tx, rx) = bounded(4);
        thread::spawn(move || {
            keep_polling(sense, poll_interval, tx);
        });
        Box::new(BackgroundSense(rx))
    }
}

fn keep_polling(
    mut sense: Box<dyn Sense>,
    poll_interval: Option<Duration>,
    sender: Sender<Result<Input, Error>>,
) {
    loop {
        match sense.poll() {
            Ok(input) => match sender.send(Ok(input)) {
                Ok(_) => (),
                Err(e) => {
                    debug!("Terminating sensor thread, remote end hung up: {:?}", e);
                    break;
                }
            },
            Err(Error::WouldBlock) => match poll_interval {
                Some(interval) => thread::sleep(interval),
                None => thread::yield_now(),
            },
            fatal => {
                match sender.send(fatal) {
                    Ok(_) => (),
                    Err(e) => debug!("Terminating sensor thread, remote end hung up: {:?}", e),
                }
                break;
            }
        }
    }
}
