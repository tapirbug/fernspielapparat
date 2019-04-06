use crate::sense::dial::Input;
use crate::sense::{Error, Sense};
use std::io::{stdin, Read};

/// A dial that reads from stdin.
pub struct Stdin {
    buf: [u8; 1],
    last_input: Option<Input>,
}

impl Sense for Stdin {
    /// Tries to get the next input from stdin, if any.
    fn poll(&mut self) -> Result<Input, Error> {
        self.buf[0] = 0;

        let next_input = match stdin().lock().read(&mut self.buf) {
            Ok(1) => {
                let next_input = parse_byte_input(self.buf[0]);
                match (self.last_input, next_input) {
                    (Some(Input::HangUp), Some(Input::HangUp)) => None, // Ignore consecutive hangups
                    (Some(Input::PickUp), Some(Input::PickUp)) => None, // Ignore consecutive pickups
                    (_, next_input) => {
                        self.last_input = next_input;
                        next_input
                    }
                }
            }
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
            buf: [0],
            last_input: None,
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
