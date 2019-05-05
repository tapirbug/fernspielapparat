mod bg;
mod dial;
mod err;
mod sense;
mod sensors;

pub use dial::Input;
pub use err::Error;
pub use sense::Sense;
pub use sensors::Sensors;

use crate::phone::Phone;
use std::sync::{Arc, Mutex};

pub fn init_sensors(phone: &Option<Arc<Mutex<Phone>>>) -> Sensors {
    let sensors = Sensors::builder().background(dial::StdinDial::new());

    let sensors = if let Some(phone) = phone.as_ref() {
        sensors.background(dial::HardwareDial::new(phone))
    } else {
        sensors
    };

    sensors.build()
}
