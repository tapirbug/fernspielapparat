use crate::sense::{Sensors, Input};
use crate::act::Actuators;
use crate::state::State;
use std::collections::HashMap;

/// A state machine modelled after a mealy machine.
pub struct Machine {
    sensors: Sensors,
    actuators: Actuators,
    states: Vec<State>,
    current_state_idx: usize
}

impl Machine {

    pub fn new(sensors: Sensors, mut actuators: Actuators) -> Self {
        let states = vec![State::default()];

        assert!(states.len() > 0, "Expected at least one state");
        //states[0].enter(&mut actuators);

        let mut machine = Machine {
            sensors,
            actuators,
            states,
            current_state_idx: 0
        };
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
        self.sensors.poll()
            .and_then(
                |input| self.transition_for(input)
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

    fn transition_for(&self, input: Input) -> Option<usize> {
        self.current_state().transition_for(input)
    }

    fn transition_to(&mut self, idx: usize) {
        let actuators = &mut self.actuators;
        // self.states[self.current_state_idx].exit(actuators);
        self.current_state_idx = idx;
        // self.states[self.current_state_idx].enter(actuators);
    }

    /*
    fn enter(&self, actuators: &mut Actuators) {
        // TODO do something
    }

    fn exit(&self, actuators: &mut Actuators) {
        actuators.transition(Vec::new())
            .expect("Exiting state failed");
    }
    */
}

#[cfg(test)]
mod test {
    use super::*;

    
}