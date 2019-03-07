
extern crate failure;
extern crate tavla;

mod err;
mod act;
mod sense;

use crate::sense::dial::{stdin_dial, Input};
use crate::act::Actuators;
use tavla::{Voice, Speech};
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), tavla::Error> {
    let mut actuators = Actuators::new();
    let voice = tavla::any_voice()?;
    let dial = stdin_dial();

    loop {
        while let Some(input) = dial.poll() {
            match input {
                Input::Digit(_) => {
                    let speech = Box::new(
                        voice.speak(format!("You typed _{}_", input.value().unwrap())).unwrap()
                    );
                    /*actuators.transition_with_makers([
                        ||voice.speak(format!("You typed _{}_", input.value().unwrap())).unwrap()
                    ].iter())?;*/
                    actuators.transition(
                        vec![speech]
                    )?;
                },
                Input::HangUp => return Ok(()),
                Input::PickUp => {
                    voice.speak("You picked up").unwrap()
                    .await_done()?;
                },
            }
        }

        sleep(Duration::from_millis(10));
    }
}
