extern crate failure;
extern crate tavla;

mod act;
mod err;
mod sense;

use crate::act::Actuators;
use crate::sense::dial::{stdin_dial, Input};
use std::thread::sleep;
use std::time::Duration;
use tavla::{Speech, Voice};

fn main() -> Result<(), tavla::Error> {
    let mut actuators = Actuators::new();
    let voice = tavla::any_voice()?;
    let dial = stdin_dial();

    loop {
        actuators.update()?;
        while let Some(input) = dial.poll() {
            println!("{:?}", input);
            match input {
                Input::Digit(_) => {
                    let speech = Box::new(
                        voice
                            .speak(format!("You typed _{}_", input.value().unwrap()))
                            .unwrap(),
                    );
                    /*actuators.transition_with_makers([
                        ||voice.speak(format!("You typed _{}_", input.value().unwrap())).unwrap()
                    ].iter())?;*/
                    actuators.transition(vec![speech])?;
                }
                Input::HangUp => return Ok(()),
                Input::PickUp => {
                    voice.speak("You picked up").unwrap().await_done()?;
                }
            }
        }

        sleep(Duration::from_millis(10));
    }
}
