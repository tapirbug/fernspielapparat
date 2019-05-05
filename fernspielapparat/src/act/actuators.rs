use crate::act::{Act, Ring, Wait};
use crate::err::compound_result;
use crate::phone::Phone;
use crate::state::State;
use failure::Error;
use log::{debug, error, warn};
use std::mem::replace;
use std::sync::{Arc, Mutex};
use tavla::{any_voice, Voice};

pub struct Actuators {
    active: Vec<Box<dyn Act>>,
    phone: Option<Arc<Mutex<Phone>>>,
}

impl Actuators {
    pub fn new(phone: &Option<Arc<Mutex<Phone>>>) -> Self {
        Actuators {
            active: Vec::new(),
            phone: phone.as_ref().map(Arc::clone),
        }
    }

    pub fn update(&mut self) -> Result<(), Error> {
        // First give every act a chance to update
        let update_errs: Vec<_> = self
            .active
            .iter_mut()
            .map(|a| a.update())
            .filter_map(Result::err)
            .collect();

        if !update_errs.is_empty() {
            error!("Actuator update failures: {:?}", update_errs);
        }

        // remove finished acts
        self.active.retain(|a| {
            let done = a.done().unwrap_or(false);
            !done
        });

        Ok(())
    }

    /// Returns `true` all acts are done or have been cancelled.
    ///
    /// Returns `false` if some actuators are still working, e.g.
    /// speech is still ongoing.
    pub fn done(&self) -> bool {
        self.active.is_empty()
    }

    pub fn transition_to(&mut self, state: &State) -> Result<(), Error> {
        self.transition(self.make_act_states(state))
    }

    fn make_act_states(&self, state: &State) -> Vec<Box<dyn Act>> {
        let mut acts: Vec<Box<dyn Act>> = vec![];

        if !state.speech().is_empty() {
            acts.push(Box::new(
                any_voice()
                    .expect("Could not load a voice")
                    .speak(state.speech())
                    .expect("Could not start speech for state"),
            ));
        }

        if let Some(duration) = state.ring_time() {
            if let Some(phone) = self.phone.as_ref() {
                acts.push(Box::new(
                    Ring::new(phone, duration).expect("Failed to start ring"),
                ))
            } else {
                // If no real bell available, do a silent bell for timeout purposes only
                acts.push(Box::new(Wait::new(duration)))
            }
        }

        acts
    }

    pub fn transition(&mut self, next_acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
        match cancel_all(&mut replace(&mut self.active, next_acts)) {
            Err(errs) => warn!("Some acts could not be cancelled: {}", errs),
            _ => (),
        };
        Ok(())
    }
}

fn cancel_all(acts: &mut Vec<Box<dyn Act>>) -> Result<(), Error> {
    compound_result(acts.into_iter().map(|a| a.cancel()))
}

impl Drop for Actuators {
    fn drop(&mut self) {
        let mut acts = &mut replace(&mut self.active, vec![]);

        match cancel_all(&mut acts) {
            Ok(_) => debug!("Actuators dropped at shutdown"),
            Err(e) => warn!("Failed to stop actuators at shutdown: {}", e),
        }

        if let Some(phone) = self.phone.take() {
            phone
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .unring()
                .unwrap_or_else(|e| warn!("Failed to unring phone at shutdown: {}", e));
        }
    }
}
