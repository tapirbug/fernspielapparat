use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;
use crate::sense::{Sense, Error, dial::Input};

pub struct BackgroundSense(Receiver<Result<Input, Error>>);

impl Sense for BackgroundSense {
    fn poll(&mut self) -> Result<Input, Error> {
        self.0.try_recv()
            .unwrap_or(Err(Error::WouldBlock))
    }
}

impl BackgroundSense {
    pub fn spawn(sense: Box<dyn Sense + Send>) -> Box<dyn Sense> {
        // 1: if last input has not been handled yet, let worker block until receiver polled
        let (tx, rx) = sync_channel(1);
        thread::spawn(move || {
            keep_polling(sense, tx);
        });
        Box::new(BackgroundSense(rx))
    }
}

fn keep_polling(mut sense: Box<dyn Sense>, sender: SyncSender<Result<Input, Error>>) {
    loop {
        match sense.poll() {
            Ok(input) => sender.send(Ok(input))
                .expect("Could not send input back"),
            Err(Error::WouldBlock) => thread::yield_now(),
            fatal => {
                sender.send(fatal);
                break;
            },
        }
    }
}
