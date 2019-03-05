use crate::act::Act;
use failure::{Error, bail};
use std::mem::replace;

pub struct Actuators {
    active: Vec<Box<dyn Act>>
}

impl Actuators {
    pub fn new() -> Self {
        Actuators {
            active: Vec::new()
        }
    }

    pub fn transition(&mut self, next_acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
        let prev_acts = replace(&mut self.active, next_acts);
        cancel_all(prev_acts)?;
        Ok(())
    }
}

fn cancel_all(acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
    let mut cancel_errors : Vec<Error> = acts.into_iter()
        .map(|mut a| a.cancel())
        .filter_map(Result::err)
        .collect();

    match cancel_errors.len() {
        0 => Ok(()),
        1 => Err(cancel_errors.remove(0)),
        _ => bail!("Multiple cancel errors: {:?}", cancel_errors)
    }
}
