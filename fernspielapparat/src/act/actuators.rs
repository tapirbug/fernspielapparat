use crate::act::Act;
use crate::err::compound_result;
use failure::{Error, bail};
use std::mem::replace;
use std::fmt::Debug;

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

    pub fn transition_with_makers<I, F, A>(&mut self, act_makers: I) -> Result<(), Error>
        where I : IntoIterator<Item = F>,
            F : FnOnce() -> A,
            A : Act + 'static + Debug
    {
        let boxed = act_makers.into_iter()
                .map(instantiate);

        self.transition_iter(
            boxed
        )
    }

    pub fn transition_iter<I>(&mut self, next_acts: I) -> Result<(), Error>
        where I : IntoIterator<Item = Box<dyn Act>> {

        self.transition(
            next_acts.into_iter()
                .collect()
        )
    }
}

fn cancel_all(acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
    compound_result(
        acts.into_iter()
            .map(|mut a| a.cancel())
    )
}

fn instantiate<F, A>(maker: F) -> Box<dyn Act>
        where F : FnOnce() -> A,
            A : Act + 'static + Debug
{
    Box::new(maker())
}
