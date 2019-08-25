//! Module for state machine events that a remote controlling
//! application may be interested in.
use crate::result::Result;
use crate::states::Symbol;

mod composite;

pub use composite::CompositeResponder;

/// State machine event for a machine state evaluates states
/// of type `S`.
#[derive(Copy, Clone)]
pub enum Event<'a, S> {
    /// A new phonebook has been loaded, the same phonebook has
    /// been reset, or normal phonebook progression caused the
    /// initial state to be reached again.
    ///
    /// The specified initial state is now current.
    #[allow(unused)] // used through type aliases, but rustc does not pick it up
    Start { initial: &'a S },
    /// The phonebook has progressed to a terminal state.
    #[allow(unused)] // used through type aliases, but rustc does not pick it up
    Finish { terminal: &'a S },
    /// User input, timeout or other conditions caused a
    /// transition from one state to another.
    ///
    /// Is delivered _before_ the `Start` and `Finish`
    /// variants.
    #[allow(unused)] // used through type aliases, but rustc does not pick it up
    Transition {
        cause: Symbol,
        from: &'a S,
        to: &'a S,
    },
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ResponderState {
    /// The responder finished its behavior or has never done anything.
    Idle,
    /// The responder still has work to do.
    Running,
}

/// Defines async behavior that reacts to state machine events,
/// e.g. text being spoken in the background or a bell ringing.
///
/// The responder may perform some kind of side effect that
/// can be continued and checked for completion with the
/// `update` method.
pub trait Responder<S> {
    /// Sets an appropriate action for the passed event.
    fn respond(&mut self, event: &Event<S>) -> Result<()>;

    /// Continues responder specific behavior and returns an enum
    /// indicating whether the behavior still has work to do.
    ///
    /// If an error makes it impossible to continue the work of
    /// the responder, an error is returned.
    ///
    /// If the behavior, e.g. text being spoken, has not finished
    /// yet, then `Ok(Running)` is returned, if never had any work
    /// or if the work has already finished, `Ok(Idle)` is returned.
    fn update(&mut self) -> Result<ResponderState> {
        Ok(ResponderState::Idle)
    }
}
