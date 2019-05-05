use crate::act::Actuators;
use crate::sense::Sensors;
use crate::state::State;
use log::debug;
use std::time::Instant;

/// A state machine modelled after a mealy machine.
pub struct Machine {
    sensors: Sensors,
    actuators: Actuators,
    states: Vec<State>,
    current_state_idx: usize,
    /// The time of the last transition and initially the startup time.
    last_enter_time: Instant,
    /// Time when it was first detected that all actuators such as speech
    /// are finished. `None` if some actuator is still working.
    current_actuators_done_time: Option<Instant>,
}

impl Machine {
    pub fn new(sensors: Sensors, actuators: Actuators, states: Vec<State>) -> Self {
        assert!(states.len() > 0, "Expected at least one state");

        let now = Instant::now();
        let mut machine = Machine {
            sensors,
            actuators,
            states,
            current_state_idx: 0,
            last_enter_time: now,
            current_actuators_done_time: None,
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
        let transition = {
            let state = self.current_state();

            // Priority 1: timeout after actuators finished on last tick
            let timeout_transition = self
                .current_actuators_done_time
                .and_then(|done_time| state.transition_for_timeout(done_time));

            // Priority 2: end transition from this tick
            let end_transition = if self.actuators.done() {
                self.current_state().transition_end()
            } else {
                None
            };

            // Priority 3: transitions from dialing
            let input_transition = self
                .sensors
                .poll()
                .and_then(|i| self.current_state().transition_for_input(i));

            timeout_transition.or(end_transition).or(input_transition)
        };

        // If anything triggered a transition, perform it.
        transition.map(|next_idx| {
            self.transition_to(next_idx);
        });
    }

    fn actuate(&mut self) {
        self.actuators
            .update()
            .expect("Failed to update actuators.");

        if self.current_actuators_done_time.is_none() && self.actuators.done() {
            self.current_actuators_done_time = Some(Instant::now());
        }
    }

    /// `true`, if a terminal state has been reached.
    fn is_terminal(&self) -> bool {
        self.current_state().is_terminal()
    }

    fn transition_to(&mut self, idx: usize) {
        self.exit();
        self.current_state_idx = idx;
        self.enter();
    }

    /// Enters the current state.
    fn enter(&mut self) {
        let state = &self.states[self.current_state_idx];
        let actuators = &mut self.actuators;
        actuators
            .transition_to(state)
            .expect("Entering state failed");

        debug!("Transition to: {}", state.name());

        self.last_enter_time = Instant::now();
        self.current_actuators_done_time = None;
    }

    /// Exits the current state.
    fn exit(&mut self) {
        self.actuators
            .transition(Vec::new())
            .expect("Exiting state failed");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::{sleep, yield_now};
    use std::time::Duration;
    use tavla::{any_voice, Speech, Voice};

    #[test]
    #[should_panic]
    fn machine_without_states() {
        Machine::new(Sensors::builder().build(), Actuators::new(&None), vec![]);
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_end_transition_target() {
        Machine::new(
            Sensors::builder().build(),
            Actuators::new(&None),
            vec![State::builder()
                .name("with illegal end transition target")
                .end(1)
                .build()],
        )
        .update();
    }

    #[test]
    fn timeout_starts_after_ringing() {
        let ring_time = Duration::from_millis(200);
        let timeout = Duration::from_millis(350);
        let expected_duration = ring_time + timeout;

        let states = vec![
            State::builder()
                .name("ringing")
                .ring_for(ring_time)
                .timeout(timeout, 1)
                .build(),
            State::builder().name("done").terminal(true).build(),
        ];

        let test_duration = time_until_done_when_no_input(states);

        let error = delta(test_duration, expected_duration);
        let tolerance = Duration::from_millis(50);
        assert!(
            error <= tolerance,
            "Timeout was more than 50ms off from expected time"
        );
    }

    #[test]
    //#[ignore]
    fn timeout_starts_after_speech() {
        let text = ".........";
        let speech_time = actual_speech_time(text);
        let timeout = Duration::from_millis(220);
        let expected_duration = speech_time + timeout;

        dbg!(speech_time);

        let test_duration = time_until_done_when_no_input(vec![
            State::builder()
                .name("speaking")
                .speech(text)
                .timeout(timeout, 1)
                .build(),
            State::builder().name("done").terminal(true).build(),
        ]);

        let error = delta(test_duration, expected_duration);
        let tolerance = Duration::from_millis(50);
        assert!(
            error <= tolerance,
            "Timeout was more than 50ms off from expected time"
        );
    }

    /// Check how long it takes to speak the given string by actually
    /// doing it and measuring.
    fn actual_speech_time(for_str: &str) -> Duration {
        let voice = any_voice().expect("Could not load voice to calculate expected timeout time");

        let speech_start = Instant::now();

        voice
            .speak(for_str)
            .expect("Failed to speak string to calculate expected timeout time")
            .await_done()
            .expect("Failed to wait for speech end");

        speech_start.elapsed()
    }

    fn time_until_done_when_no_input(states: Vec<State>) -> Duration {
        let test_start = Instant::now();

        let mut machine = Machine::new(Sensors::builder().build(), Actuators::new(&None), states);

        while machine.update() {
            yield_now()
        } // Run until finished
        test_start.elapsed() // And report how long it took
    }

    fn delta(duration0: Duration, duration1: Duration) -> Duration {
        if duration0 > duration1 {
            duration0 - duration1
        } else {
            duration1 - duration0
        }
    }
}
