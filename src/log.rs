use failure::Error;
use log::{debug, error, LevelFilter};

#[cfg(test)]
use std::sync::Once;

#[cfg(test)]
static INIT_TEST_LOGGING: Once = Once::new();

/// Initializes logging for normal operation.
///
/// If fails, prints a message once and then never logs anything.
pub fn init_logging(verbosity_level: Option<u64>) {
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

/// Initializes logging output for test builds.
#[cfg(test)]
pub fn init_test_logging() {
    INIT_TEST_LOGGING.call_once(|| {
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .is_test(true)
            .init();
    })
}

/// Logs that the given error is fatal and leads to termination
/// of the application.
///
/// The whole error chain is printed.
///
/// Debug builds also print the stack trace.
pub fn log_fatal(error: &Error) {
    error!("Exiting due to fatal error.");
    log_backtrace(error);
    log_causes(error);
}

pub fn log_backtrace(error: &Error) {
    debug!("Backtrace: {}", error.backtrace());
}

pub fn log_causes(error: &Error) {
    for cause in error.iter_chain() {
        error!("Cause: {}", cause);
        debug!("Cause: {:?}", cause);
    }
}
