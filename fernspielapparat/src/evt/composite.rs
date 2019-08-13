use super::*;

use crate::err::compound_result;

pub struct CompositeResponder<S>(Vec<Box<dyn Responder<S>>>);

impl<S> CompositeResponder<S> {
    pub fn from(responders: Vec<Box<dyn Responder<S>>>) -> Self {
        CompositeResponder(responders)
    }
}

impl<S> Responder<S> for CompositeResponder<S> {
    fn respond(&mut self, event: &Event<S>) -> Result<()> {
        compound_result(self.0.iter_mut().map(|r| r.respond(event)))
    }
}
