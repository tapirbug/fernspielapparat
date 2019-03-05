use crate::sense::dial::Input;

use std::io::{stdin, Read};
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};

/// A dial that reads from stdin without buffering.
pub struct Dial(Receiver<Input>);

impl Dial {
    /// Spawns a background worker that listens to stdin.
    pub fn new() -> Dial {
        let (tx, rx) = channel();
        thread::spawn(move || {
            read_inputs(tx);
        });
        Self(rx)
    }

    /// Tries to get the next input from stdin, if any.
    /// Returns None forever when the remote end encountered an error.
    pub fn poll(&self) -> Option<Input> {
        self.0.try_recv().ok()
    }
}

fn read_inputs(sender: Sender<Input>) {
    let stdin = stdin();
    let mut stdin = stdin.lock();
        
    let mut buffer = [0; 1];
    loop {
        let next_input = match stdin.read(&mut buffer) {
            Ok(1) => parse_byte_input(buffer[0]),
            // This catches errors on windows for UTF-8, or when non-blocking IO
            // Also catches Ok(0)
            _ => None
        };

        if let Some(next_input) = next_input {
            sender.send(next_input)
                .expect("Failed to deliver stdin input");
        }
    }
}

fn parse_byte_input(byte: u8) -> Option<Input> {
    match byte {
        digit@b'0'..=b'9' => Input::digit(digit - b'0').ok(),
        // Pick up, take or something
        b'p' | b't' => Some(Input::pick_up()),
        // Hang up, return or something
        b'h' | b'r' => Some(Input::hang_up()),
        // Ignore any other one-byte UTF-8 character
        _ => None
    }
}
