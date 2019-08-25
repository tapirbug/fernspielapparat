//! Core functionality used by the runner in `main.js`
//! and also for headless integration tests.
//!
//! Exports `AppBuilder`, `App` and `Phone` as the only
//! interface to the core functionality for client code.

#![feature(drain_filter)]

#[cfg(test)]
mod testutil;

mod acts;
mod err;
mod evt;
mod phone;
mod result;
mod senses;
mod serve;
mod states;
mod util;

pub mod app;
pub mod books;
pub mod check;
pub mod log;

pub use app::{App, Builder as AppBuilder};
pub use phone::Phone;
