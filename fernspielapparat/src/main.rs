
extern crate failure;
extern crate tavla;

mod act;
mod sense;

use crate::sense::dial::{stdin_dial, Input};
use tavla::{Voice, Speech};
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), tavla::Error> {
    let voice = tavla::any_voice()?;
    let dial = stdin_dial();

    loop {
        while let Some(input) = dial.poll() {
            match input {
                Input::Digit(_) => {
                    voice.speak(format!("You typed _{}_", input.value().unwrap()))?
                        .await_done()?;
                },
                Input::HangUp => return Ok(()),
                Input::PickUp => (),
            }
        }

        sleep(Duration::from_millis(10));
    }
}
