use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use crate::sense::{Sense, Error, dial::Input};

pub struct BackgroundSense(Receiver<Input>);

impl Sense for BackgroundSense {
    fn poll(&mut self) -> Result<Input, Error> {
        self.0.try_recv()
            .or_else(|_| Err(Error::WouldBlock))
    }
}

impl BackgroundSense {
    pub fn spawn(sense: Box<dyn Sense + Send>) -> Box<dyn Sense> {
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