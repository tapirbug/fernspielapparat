use crate::act::{Act, Ring};
use crate::err::compound_result;
use crate::phone::Phone;
use crate::state::State;
use failure::Error;
use log::{error, warn};
use std::fmt::Debug;
use std::mem::replace;
use std::sync::{Arc, Mutex};
use tavla::{any_voice, Voice};

pub struct Actuators {
    active: Vec<Box<dyn Act>>,
    phone: Option<Arc<Mutex<Phone>>>,
}

#[allow(dead_code)]
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

        if let Some(phone) = self.phone.as_ref() {
            if let Some(duration) = state.ring_time() {
                acts.push(Box::new(
                    Ring::new(phone, duration).expect("Failed to start ring"),
                ))
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

    pub fn transition_with_makers<I, F, A>(&mut self, act_makers: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = F>,
        F: FnOnce() -> A,
        A: Act + 'static + Debug,
    {
        let boxed = act_makers.into_iter().map(instantiate);

        self.transition_iter(boxed)
    }

    pub fn transition_iter<I>(&mut self, next_acts: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Box<dyn Act>>,
    {
        self.transition(next_acts.into_iter().collect())
    }
}

fn cancel_all(acts: &mut Vec<Box<dyn Act>>) -> Result<(), Error> {
    compound_result(acts.into_iter().map(|a| a.cancel()))
}

fn instantiate<F, A>(maker: F) -> Box<dyn Act>
where
    F: FnOnce() -> A,
    A: Act + 'static + Debug,
{
    Box::new(maker())
}
