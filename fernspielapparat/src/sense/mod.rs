pub mod dial;

mod bg;
mod sense;
mod sensors;

pub use sense::{Sense, Error};
pub use sensors::Sensors;

pub fn init_sensors() -> Sensors {
    Sensors::builder()
        .background(dial::stdin_dial())
        .build()
}