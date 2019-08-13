use crate::senses::Input;
use std::time::Duration;

/// A symbol of the input alphabet to the state machine.
#[derive(Debug, Clone, Copy)]
pub enum Symbol {
    /// Emitted once when receiving input from the hardware phone.
    Dial(Input),
    /// Emitted when all actuators are done with the duration
    /// indicating how long this condition is already true.
    Done(Duration),
}
