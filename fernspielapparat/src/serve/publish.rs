use super::Server;

use crate::evt::{Event, Responder};
use crate::states::State;

use failure::Error;

use std::rc::Rc;

pub struct EventPublisher(Rc<Server>);

impl EventPublisher {
    pub fn through(server: &Rc<Server>) -> Self {
        EventPublisher(Rc::clone(server))
    }
}

impl Responder<State> for EventPublisher {
    fn respond(&mut self, event: &Event<State>) -> Result<(), Error> {
        self.0.publish(event.into());
        Ok(())
    }
}
