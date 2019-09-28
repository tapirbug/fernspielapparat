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
const DEFAULT_ADDRESS: &str = "0.0.0.0";
const DEFAULT_PORT: &str = "38397";

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
                .help("Phone book to run at startup")
                .long_help("Path to a phone book to load and run at startup.")
                .required_unless_one(&["serve", "serve_address", "serve_port", "demo", "test"])
                .conflicts_with("demo")
                .conflicts_with("test"),
        )
        .arg(
            Arg::with_name("serve")
                .short("s")
                .long("serve")
                .help("Host WebSockets server for remote control")
                .long_help(&format!(
                    "Starts up a WebSockets server for remote control, \
                     executing in the background. \
                     Hosts on {address}:{port} per default. \
                     See --addr and --port to override bind address or port. \
                     Any phonebook provided via path is executed at startup. \
                     Without a startup phonebook, the runtime remains silent until \
                     a phonebook has been uploaded via remote control.",
                    address = DEFAULT_ADDRESS,
                    port = DEFAULT_PORT
                ))
                .conflicts_with("test"),
        )
        .arg(
            Arg::with_name("serve_address")
                .help("WebSockets server bind address")
                .long_help(&format!(
                    "Sets the bind address to host a WebSockets server for remote control on. \
                     Implies --serve. \
                     Defaults to {addr}, if --serve is used without an explicit address.",
                    addr = DEFAULT_ADDRESS
                ))
                .short("a")
                .long("addr")
                .takes_value(true)
                .value_name("ADDRESS")
                .default_value_if("serve", None, DEFAULT_ADDRESS)
                .default_value_if("serve_port", None, DEFAULT_ADDRESS),
        )
        .arg(
            Arg::with_name("serve_port")
                .help("WebSockets server bind port")
                .long_help(&format!(
                    "Sets the port to host a WebSockets server for remote control on. \
                     Implies --serve. \
                     Defaults to {port}, if --serve is used without an explicit port.",
                    port = DEFAULT_PORT
                ))
                .short("p")
                .long("port")
                .takes_value(true)
                .value_name("PORT")
                .default_value_if("serve", None, DEFAULT_PORT)
                .default_value_if("serve_address", None, DEFAULT_PORT),
        )
        .arg(
            Arg::with_name("demo")
                .short("d")
                .long("demo")
                .help("Loads a demo phonebook instead of a file")
                .long_help("Loads a demo phonebook instead of a file."),
        )
        .arg(
            Arg::with_name("exit-on-terminal")
                .long("exit-on-terminal")
                .help("Terminate when reaching terminal state")
                .long_help(
                    "Instead of starting over, exit with status 0 when reaching a terminal state.",
                ),
        )
        .arg(
            Arg::with_name("test")
                .short("t")
                .long("test")
                .help("Perform hardware and speech synth check, then exit")
                .long_help(
                    "Lets the phone ring and speak for one second as a basic hardware \
                     check, tries to speak a sentence through speech synthesis, then exits.",
                ),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Silence warnings and errors")
                .long_help("Turn off logging completely, including warnings and errors."),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Verbose logging")
                .long_help(
                    "Print non-essential output with diagnostic information to stderr. \
                     Multiple occurrences increase logging verbosity. -vvv is the highest verbosity, \
                     printing debug information."
                )
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
            Ok(_) => debug!("exiting after normal operation."),
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

    let some_serve_arg_present = matches.is_present("serve")
        || matches.occurrences_of("serve_address") > 0
        || matches.occurrences_of("serve_port") > 0;
    if some_serve_arg_present {
        let bind_address = matches
            .value_of("serve_address")
            // unwrap is safe: 127.0.0.1 is specified as default value
            .unwrap();
        let bind_port = matches
            .value_of("serve_port")
            // unwrap is safe: 38397 is specified as default value
            .unwrap();
        let bind_to = &format!("{addr}:{port}", addr = bind_address, port = bind_port);

        debug!(
            "starting WebSockets remote control server on {bind_to}",
            bind_to = bind_to
        );

        app.serve(bind_to)?;
    }

    Ok(app.build()?)
}
