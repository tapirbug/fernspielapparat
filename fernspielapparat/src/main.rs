extern crate clap;
extern crate crossbeam_channel;
extern crate cute_log;
extern crate failure;
extern crate i2c_linux;
extern crate log;
extern crate serde;
extern crate serde_yaml;
extern crate tavla;

mod act;
mod book;
mod err;
mod phone;
mod sense;
mod state;

use crate::act::Actuators;
use crate::phone::Phone;
use crate::sense::init_sensors;
use crate::state::{Machine, State};
use clap::{crate_authors, crate_name, crate_version, App, Arg};
use failure::Error;
use log::{debug, error, info, warn, LevelFilter};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    if bootstrap().is_err() {
        exit(1);
    }
}

fn bootstrap() -> Result<(), Error> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about("Runtime environment for fernspielapparat phonebooks.")
        .author(crate_authors!())
        .arg(
            Arg::with_name("phonebook")
                .help("Path to the phone book to use")
                .required(true)
                .conflicts_with("default"),
        )
        .arg(
            Arg::with_name("default")
                .short("d")
                .long("default")
                .help("Loads the default phonebook at startup")
                .conflicts_with("phonebook"),
        )
        .arg(Arg::with_name("test").short("t").long("test").help(
            "Lets the phone ring and speak for one second as a basic hardware \
             check, then exits.",
        ))
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Silence warnings and errors"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Print non-essential output with diagnostic information to stderr")
                .conflicts_with("quiet"),
        )
        .get_matches();

    let verbosity_level = if matches.is_present("quiet") {
        None
    } else {
        Some(matches.occurrences_of("verbose"))
    };
    init_logging(verbosity_level);

    if matches.is_present("test") {
        check_phone()
    } else {
        let states = if matches.is_present("default") {
            book::from_str(include_str!("../resources/default.yaml"))?
        } else {
            book::from_path(matches.value_of("phonebook").unwrap())?
        };

        debug!("{:?}", &states);

        let result = launch(states);
        match result {
            Ok(_) => debug!("Exiting after normal operation."),
            Err(ref err) => log_error(err),
        }
        result
    }
}

fn launch(states: Vec<State>) -> Result<(), Error> {
    let phone = Phone::new().ok().map(|p| Arc::new(Mutex::new(p)));

    if phone.is_some() {
        info!("Phone connected, starting normal operation.");
    } else {
        warn!("No phone available.");
    }

    let actuators = Actuators::new(&phone);
    let sensors = init_sensors(&phone);
    let mut machine = Machine::new(sensors, actuators, states);

    while machine.update() {
        sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn log_error(error: &Error) {
    error!("Exiting due to fatal error.");
    debug!("Backtrace: {}", error.backtrace());

    for cause in error.iter_chain() {
        error!("Cause: {}", cause);
        debug!("Cause: {:?}", cause);
    }
}

fn check_phone() -> Result<(), Error> {
    info!("Testing communication with hardware phone...");

    let test_result = Phone::new().and_then(|mut phone| {
        phone.ring()?;
        sleep(Duration::from_secs(1));
        phone.unring()?;
        Ok(())
    });

    match test_result {
        Ok(_) => info!("Hardware phone ok."),
        Err(ref e) => {
            error!("Communication with hardware phone failed: {}.", e);
        }
    }

    Ok(test_result?)
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
