extern crate clap;
extern crate cute_log;
extern crate failure;
extern crate i2c_linux;
extern crate log;
extern crate tavla;

mod act;
mod err;
mod phone;
mod sense;

use crate::act::{Act, Actuators, Ring};
use crate::phone::Phone;
use crate::sense::{Input, init_sensors};
use clap::{crate_authors, crate_name, crate_version, App, Arg};
use failure::Error;
use log::{debug, error, warn, info, LevelFilter};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use std::process::exit;
use tavla::Voice;

fn main() {
    if run().is_err() {
        exit(1);
    }
}

fn run() -> Result<(), Error> {
    let matches = App::new(crate_name!())
       .version(crate_version!())
       .about("Runtime environment for fernspielapparat phonebooks.")
       .author(crate_authors!())
       .arg(Arg::with_name("test")
            .short("t")
            .long("test")
            .help("Before starting main operation, lets the phone ring and speak for one second as a basic hardware check"))
        .arg(Arg::with_name("quiet")
            .short("q")
            .long("quiet")
            .help("Silence warnings and errors"))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .multiple(true)
            .help("Print non-essential output with diagnostic information to stderr")
            .conflicts_with("quiet"))
       .get_matches();

    let verbosity_level = if matches.is_present("quiet") {
        None
    } else {
        Some(matches.occurrences_of("verbose"))
    };
    init_logging(verbosity_level);

    if matches.is_present("test") {
        check_phone();
    }

    let result = launch_runtime();
    match result {
        Ok(_) => debug!("Exiting after normal operation."),
        Err(ref err) => {
            error!("Exiting due to fatal error: {}", err);
        }
    }

    result
}

fn launch_runtime() -> Result<(), Error> {
    let phone = Phone::new()
        .ok()
        .map(|p| Arc::new(Mutex::new(p)));

    if phone.is_some() {
        info!("Phone connected, starting normal operation.");
    } else {
        warn!("No phone available.");
    }
    
    let mut actuators = Actuators::new();
    let mut sensors = init_sensors(&phone);
    let voice = tavla::any_voice()?;

    loop {
        while let Some(input) = sensors.poll() {
            debug!("{:?}", input);
            match input {
                Input::Digit(_) => {
                    let speech = Box::new(
                        voice
                            .speak(format!("You typed _{}_", input.value().unwrap_or(100)))
                            .unwrap(),
                    );
                    actuators.transition(vec![speech])?;
                }
                Input::HangUp => return Ok(()),
                Input::PickUp => {
                    let mut acts: Vec<Box<dyn Act>> = Vec::new();
                    if let Some(phone) = phone.as_ref() {
                        acts.push(Box::new(
                            Ring::new(&phone, Duration::from_millis(100))?
                        ));
                    }
                    acts.push(Box::new(voice.speak("You picked up").unwrap()));
                    actuators.transition(acts)?;
                }
            }
        }

        actuators.update()?;

        sleep(Duration::from_millis(10));
    }
}

fn check_phone() {
    info!("Testing communication with hardware phone...");

    let test_result = Phone::new().and_then(|mut phone| {
        phone.ring()?;
        sleep(Duration::from_secs(1));
        phone.unring()
    });

    match test_result {
        Ok(_) => info!("Hardware phone ok."),
        Err(e) => {
            error!("Communication with hardware phone failed: {}.", e);
        }
    }
}

fn init_logging(verbosity_level: Option<u64>) {
    let level = match verbosity_level {
        None => LevelFilter::Off,
        Some(0) => LevelFilter::Warn,
        Some(1) => LevelFilter::Info,
        Some(2) => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let res = cute_log::init_with_max_level(level);
    if let Err(err) = res {
        eprintln!(
            "Failed to initialize logging. Will stay silent for the rest of execution. Error: {}",
            err
        )
    }
}
