extern crate clap;
extern crate failure;
extern crate tavla;
extern crate i2c_linux;

mod act;
mod err;
mod sense;
mod phone;

use crate::act::Actuators;
use crate::sense::{Sensors, init_sensors, dial::Input};
use crate::phone::Phone;
use std::thread::sleep;
use std::time::Duration;
use tavla::{Speech, Voice};
use clap::{App, Arg, crate_name, crate_version, crate_authors};

fn main() -> Result<(), tavla::Error> {
    let matches = App::new(crate_name!())
       .version(crate_version!())
       .about("Runtime environment for fernspielapparat phonebooks.")
       .author(crate_authors!())
       .arg(Arg::with_name("test")
            .short("t")
            .long("config")
            .help("Before starting main operation, lets the phone ring and speak for one second"))
       .get_matches(); 

    if matches.is_present("test") {
        println!("Testing communication with hardware phone...");

        let test_result = Phone::new()
            .and_then(|mut phone| {
                phone.ring()?;
                sleep(Duration::from_secs(1));
                phone.unring()
            });

        match test_result {
            Ok(_) => {
                println!("Hardware phone ok.")
            }
            Err(e) => {
                println!("error: Communication with hardware phone failed: {}.", e);
            } 
        }

        println!("Test successful.");
    }

    let mut actuators = Actuators::new();
    let mut sensors = init_sensors();
    let voice = tavla::any_voice()?;

    loop {
        actuators.update()?;
        while let Some(input) = sensors.poll() {
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
