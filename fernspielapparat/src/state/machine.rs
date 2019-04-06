use crate::sense::{Sensors, Input};
use crate::act::{Actuators, Act};
use crate::state::State;
use std::time::Instant;
use log::debug;

/// A state machine modelled after a mealy machine.
pub struct Machine {
    sensors: Sensors,
    actuators: Actuators,
    states: Vec<State>,
    current_state_idx: usize,
    last_enter_time: Instant
}

impl Machine {

    pub fn new(sensors: Sensors, mut actuators: Actuators, states: Vec<State>) -> Self {
        assert!(states.len() > 0, "Expected at least one state");

        let mut machine = Machine {
            sensors,
            actuators,
            states,
            current_state_idx: 0,
            last_enter_time: Instant::now()
        };
        machine.enter();
        machine
    }

    /// Starts the next cycle of the machine, first polling
    /// input and changing state if necessary, then setting
    /// the state of actuators.
    /// 
    /// Returns `true` if still running, `false` only if a
    /// terminal state has been reached.
    pub fn update(&mut self) -> bool {
        if self.current_state().is_terminal() {
            // Exit early if already done
            return false;
        }

        self.sense();
        self.actuate();

        !self.is_terminal()
    }

    fn current_state(&self) -> &State {
        &self.states[self.current_state_idx]
    }

    /// Accepts the next input from actuators and changes state
    /// if a transition is defined.
    fn sense(&mut self) {
        self.current_state().transition_for_timeout(self.last_enter_time)
            .or_else(|| self.sensors.poll()
                    .and_then(|i| self.current_state().transition_for_input(i))
            )
            .map(|next_idx| {
                self.transition_to(next_idx);
            });
    }


    fn actuate(&mut self) {
        self.actuators.update()
            .expect("Failed to update actuators.");
    }

    /// `true`, if a terminal state has been reached.
    fn is_terminal(&self) -> bool {
        self.current_state().is_terminal()
    }

    fn transition_to(&mut self, idx: usize) {
        let actuators = &mut self.actuators;
        self.exit();
        self.current_state_idx = idx;
        self.enter();
    }

    /// Enters the current state.
    fn enter(&mut self) {
        let state = &self.states[self.current_state_idx];
        let actuators = &mut self.actuators;
        actuators.transition_to(state)
            .expect("Entering state failed");

        debug!("Transition to: {}", state.name());

        self.last_enter_time = Instant::now();
    }

    /// Exits the current state.
    fn exit(&mut self) {
        self.actuators.transition(Vec::new())
            .expect("Exiting state failed");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    
}