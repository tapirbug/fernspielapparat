use super::*;

use crate::err::{compound_error, compound_result};

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

    fn update(&mut self) -> Result<ResponderState> {
        let mut compound_state = ResponderState::Idle;
        let mut errs = Vec::new();

        for responder in &mut self.0 {
            match responder.update() {
                Ok(ResponderState::Idle) => (),
                Ok(ResponderState::Running) => compound_state = ResponderState::Running,
                Err(e) => errs.push(e),
            }
        }

        compound_error(errs).map(|_| compound_state)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone)]
    struct ResponderWithState(ResponderState);

    impl Responder<()> for ResponderWithState {
        fn respond(&mut self, _: &Event<()>) -> Result<()> {
            Ok(())
        }

        fn update(&mut self) -> Result<ResponderState> {
            Ok(self.0)
        }
    }

    #[test]
    fn aggregate_state() {
        // given
        let idle = ResponderWithState(ResponderState::Idle);
        let running = ResponderWithState(ResponderState::Running);

        // when
        let idle_and_idle =
            CompositeResponder::from(vec![Box::new(idle.clone()), Box::new(idle.clone())])
                .update()
                .unwrap();
        let idle_and_running =
            CompositeResponder::from(vec![Box::new(idle.clone()), Box::new(running.clone())])
                .update()
                .unwrap();
        let running_and_idle =
            CompositeResponder::from(vec![Box::new(running.clone()), Box::new(idle.clone())])
                .update()
                .unwrap();
        let running_and_running =
            CompositeResponder::from(vec![Box::new(running.clone()), Box::new(running.clone())])
                .update()
                .unwrap();

        // then
        assert_eq!(idle_and_idle, ResponderState::Idle);
        assert_eq!(idle_and_running, ResponderState::Running);
        assert_eq!(running_and_idle, ResponderState::Running);
        assert_eq!(running_and_running, ResponderState::Running);
    }
}
