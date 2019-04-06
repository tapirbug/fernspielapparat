use crate::sense::Input;
use builder::StateBuilder;

#[derive(Default)]
pub struct State {
    speech: String
}

impl State {
    pub fn builder() -> StateBuilder {
        StateBuilder::new()
    }

    /// Returns a transition target ID or `None` for no
    /// transition.
    pub fn transition_for(&self, input: Input) -> Option<usize> {
        unimplemented!()
    }

    pub fn is_terminal(&self) -> bool {
        false
    }
}

mod builder {
    use super::State;

    pub struct StateBuilder {
        state: State
    }

    impl StateBuilder {
        pub fn new() -> Self {
            StateBuilder {
                state: Default::default()
            }
        }

        pub fn speech(mut self, speech: String) -> Self {
            self.state.speech = speech;
            self
        }
    }
}