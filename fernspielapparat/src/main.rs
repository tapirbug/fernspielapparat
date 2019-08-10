//! Parses command line arguments and sets the ball rolling by building
//! and launching the `App`, setting it up so it gracefully returns on
//! ctrl+c.
//!
//! Handles exit codes based on whether the `App` produced an error when
//! run.
//!
//! Calls into the respective modules to set up logging and ensures fatal
//! errors are being logged.
//!
//! Also provides CLI access to the hardware check.
use clap::{self, crate_authors, crate_name, crate_version, Arg, ArgMatches};
use failure::Error;
use fernspielapparat::{
    books,
    check::check_system,
    log::{init_logging, log_fatal},
    App,
};
use log::{debug, info, warn};
use std::process::exit;

/// When `--serve` is used without a bind point, use this.
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:38397";

fn main() {
    if bootstrap().is_err() {
        exit(1);
    }
}

fn bootstrap() -> Result<(), Error> {
    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .about("Runtime environment for fernspielapparat phonebooks.")
        .author(crate_authors!())
        .arg(
            Arg::with_name("phonebook")
                .help("Path to the phone book to use")
                .required(true)
                .conflicts_with("demo")
                .conflicts_with("test")
                .conflicts_with("serve"),
        )
        .arg(
            Arg::with_name("serve")
                .short("s")
                .long("serve")
                .takes_value(true)
                .default_value(DEFAULT_BIND_ADDRESS)
                .value_name("HOSTNAME_AND_PORT")
                .hide_default_value(true)
                .help(&format!(
                    "Starts up a WebSockets server for remote control on the \
                     specified hostname and port, or \"{default}\" if no value specified.",
                    default = DEFAULT_BIND_ADDRESS
                )),
        )
        .arg(
            Arg::with_name("demo")
                .short("d")
                .long("demo")
                .help("Loads a demo phonebook instead of a file"),
        )
        .arg(
            Arg::with_name("exit-on-terminal")
                .long("exit-on-terminal")
                .help(
                    "Instead of starting over, exit with status 0 when reaching a terminal state.",
                ),
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
        check_system()
    } else {
        let result = build_app(matches).and_then(|mut a| {
            debug!("initialization complete, starting");
            a.run()
        });

        match result {
            Ok(_) => debug!("Exiting after normal operation."),
            Err(ref err) => log_fatal(err),
        }

        result
    }
}

fn build_app(matches: ArgMatches) -> Result<App, Error> {
    let mut app = App::builder();

    if matches.is_present("demo") || matches.is_present("phonebook") {
        app.startup_phonebook(if matches.is_present("demo") {
            books::from_str(include_str!("../resources/demo.yaml"))?
        } else {
            books::from_path(matches.value_of("phonebook").unwrap_or(""))?
        });
    }

    app.terminate_on_ctrlc_and_sigterm();

    if matches.is_present("exit-on-terminal") {
        app.exit_on_terminal_state();
    } else {
        app.rewind_on_terminal_state();
    }

    match app.phone("/dev/i2c-1", 4) {
        Ok(_) => info!("phone connected on dev/i2c-1, address 4."),
        Err(e) => warn!("no phone available, error: {}", e),
    }

    if matches.occurrences_of("serve") > 0 {
        let bind_to = matches
            .value_of("serve")
            // unwrap is safe: 127.0.0.1:38397 is specified as default value
            .unwrap();

        debug!("starting WebSockets remote control server on {}", bind_to);

        app.serve(bind_to)?;
    }

    Ok(app.build()?)
}
