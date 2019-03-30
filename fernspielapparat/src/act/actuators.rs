use crate::act::Act;
use crate::err::compound_result;
use failure::Error;
use log::error;
use std::fmt::Debug;
use std::mem::replace;

pub struct Actuators {
    active: Vec<Box<dyn Act>>,
}

#[allow(dead_code)]
impl Actuators {
    pub fn new() -> Self {
        Actuators { active: Vec::new() }
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

    pub fn transition(&mut self, next_acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
        cancel_all(&mut replace(&mut self.active, next_acts))?;
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
