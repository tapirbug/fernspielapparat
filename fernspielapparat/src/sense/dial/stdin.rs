use crate::sense::dial::Input;
use crate::sense::{Sense, Error};
use std::io::{stdin, Read, StdinLock};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// A dial that reads from stdin.
pub struct Stdin {
    buf: [u8; 1]
}

impl Sense for Stdin {
    /// Tries to get the next input from stdin, if any.
    fn poll(&mut self) -> Result<Input, Error> {
        let next_input = match stdin().lock().read(&mut self.buf) {
            Ok(1) => parse_byte_input(self.buf[0]),
            // This catches errors on windows for UTF-8, or when non-blocking IO
            // Also catches Ok(0)
            _ => None,
        };

        if let Some(next_input) = next_input {
            Ok(next_input)
        } else {
            Err(Error::WouldBlock)
        }
    }
}

impl Stdin {
    /// Locks on stdin.
    pub fn new() -> Stdin {
        Stdin {
            buf: [0]
        }
    }
}

fn parse_byte_input(byte: u8) -> Option<Input> {
    match byte {
        digit @ b'0'..=b'9' => Input::digit(digit - b'0').ok(),
        // Pick up, take or something
        b'p' | b't' => Some(Input::pick_up()),
        // Hang up, return or something
        b'h' | b'r' => Some(Input::hang_up()),
        // Ignore any other one-byte UTF-8 character
        _ => None,
    }
}
