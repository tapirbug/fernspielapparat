use super::sym::Symbol;

use crate::evt::{Event as EventForState, Responder, ResponderState};
use crate::senses::Sensors;
use crate::states::State;

use failure::Error;
use log::{debug, error};

use std::mem::replace;
use std::time::Instant;

type Result<T> = std::result::Result<T, Error>;
type Event<'a> = EventForState<'a, State>;

/// A state machine modelled after a mealy machine.
pub struct Machine<R> {
    sensors: Sensors,
    responder: R,
    states: Vec<State>,
    current_state_idx: usize,
    /// The time of the last transition and initially the startup time.
    last_enter_time: Instant,
    last_responder_state: ResponderState,
    /// Time when it was first detected that all actuators such as speech
    /// are finished. `None` if some actuator is still working.
    responder_done_time: Option<Instant>,
}

impl<R: Responder<State>> Machine<R> {
    pub fn new(sensors: Sensors, responder: R, states: &[State]) -> Self {
        let now = Instant::now();
        let mut machine = Machine {
            sensors,
            responder,
            states: states.to_vec(),
            current_state_idx: 0,
            last_enter_time: now,
            // consider running until end of first update
            last_responder_state: ResponderState::Running,
            responder_done_time: None,
        };
        machine.init();
        machine
    }

    fn init(&mut self) {
        assert!(!self.states.is_empty(), "Expected at least one state");

        self.reset(); // some redundant work on first init, but needed on load
        if let Err(err) = self.enter() {
            error!("Failed to enter initial state: {}", err);
        }
    }

    /// Terminates this machine and returns a new machine with the
    /// given responder and states, re-using the sensors that were
    /// used by the terminated machine.
    pub fn load(&mut self, responder: R, states: &[State]) {
        // hack: temporarily set dummy sensors and move the real ones out
        let sensors = replace(&mut self.sensors, Sensors::blind());

        // Then overwrite self with newly initialized machine,
        // re-using the old sensors
        *self = Machine::new(sensors, responder, states);
    }

    pub fn reset(&mut self) {
        self.current_state_idx = 0;
        self.last_enter_time = Instant::now();
        self.responder_done_time = None;
        // consider running until end of first update after reset
        self.last_responder_state = ResponderState::Running;

        // let actuators react to reset or load
        let initial = &self.states[self.current_state_idx];
        self.responder
            .respond(&Event::Start { initial })
            .unwrap_or_else(|e| {
                error!(
                    "failed to react to transition to initial state, \
                     continuing to run, error: {}",
                    e
                )
            });

        if initial.is_terminal() {
            // initial state is also terminal state,
            // immediately send the finish event
            self.responder
                .respond(&Event::Finish { terminal: initial })
                .unwrap_or_else(|e| {
                    error!(
                        "failed to react to immediate finish of  \
                         initial state, continuing to run, error: {}",
                        e
                    )
                });
        }

        // sensors cannot be reset

        if let Err(err) = self.enter() {
            error!("Failed enter initial state after reset: {}", err);
        }
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

        // First ensure that finished actuators are picked up
        self.actuate();

        // Then read inputs and perform transitions as necessary
        if let Err(err) = self.sense() {
            error!("Error when processing input: {}", err);
        }

        !self.is_terminal()
    }

    fn current_state(&self) -> &State {
        &self.states[self.current_state_idx]
    }

    fn in_initial_state(&self) -> bool {
        self.current_state_idx == 0
    }

    fn responder_done(&self) -> bool {
        match self.last_responder_state {
            ResponderState::Idle => true,
            _ => false,
        }
    }

    /// Accepts the next input from sensors and changes state
    /// if a transition is defined.
    ///
    /// If a transition ocurred, returns the causing symbol
    /// and a reference to that state.
    fn sense(&mut self) -> Result<()> {
        // Read the next symbol and form a pair with a transition target.
        let transition = self
            .poll_input()
            .and_then(|i| self.find_transition(&i).map(|t| (i, t)));

        // If anything triggered a transition, perform it.
        if let Some((symbol, next_idx)) = transition {
            self.transition_to(symbol, next_idx)?;
        }

        Ok(())
    }

    fn poll_input(&mut self) -> Option<Symbol> {
        self.sensors
            .poll()
            .map(Symbol::Dial)
            // timeouts are only considered when there is no simultaneous input
            .or_else(|| self.responder_done_time.map(|t| Symbol::Done(t.elapsed())))
    }

    /// Finds a transition target index that should be transitioned to
    /// after reading the given symbol.
    fn find_transition(&mut self, symbol: &Symbol) -> Option<usize> {
        let state = self.current_state();
        match symbol {
            // Priority 1: transitions from dialing in this tick
            Symbol::Dial(input) => state.transition_for_input(*input),
            Symbol::Done(duration) => {
                // Priority 2: timeout with time value
                state
                    .transition_for_timeout(duration)
                    // Priority 3: end transition from last tick
                    .or_else(|| state.transition_end())
            }
        }
    }

    fn actuate(&mut self) {
        self.last_responder_state = self.responder.update().unwrap_or_else(|e| {
            error!(
                "failed to update actuators, \
                 continuing and considering them as finished, error: {}",
                e
            );
            ResponderState::Idle
        });

        if self.responder_done_time.is_none() && self.responder_done() {
            debug!("Actuators done: {:?}", self.current_state().name());
            self.responder_done_time = Some(Instant::now());
        }
    }

    /// `true`, if a terminal state has been reached.
    pub fn is_terminal(&self) -> bool {
        self.current_state().is_terminal()
    }

    fn transition_to(&mut self, cause: Symbol, idx: usize) -> Result<()> {
        let prev_idx = self.current_state_idx;
        self.current_state_idx = idx;

        self.respond_to_transition(cause, prev_idx, idx)
            .unwrap_or_else(|e| {
                error!(
                    "failed to react to transition, \
                     continuing to run, error: {}",
                    e
                )
            });

        self.enter()
    }

    fn respond_to_transition(&mut self, cause: Symbol, from: usize, to: usize) -> Result<()> {
        let from = &self.states[from];
        let to = &self.states[to];

        // first the generic transition event
        self.responder
            .respond(&Event::Transition { cause, from, to })?;

        // then specialized for initial/terminal, only if transition evt did not err
        if self.in_initial_state() {
            self.responder.respond(&Event::Start { initial: to })
        } else if to.is_terminal() {
            self.responder.respond(&Event::Finish { terminal: to })
        } else {
            Ok(())
        }
    }

    /// Enters the current state.
    fn enter(&mut self) -> Result<()> {
        self.last_enter_time = Instant::now();
        self.responder_done_time = None;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::acts::Actuators;
    use crate::testutil::assert_duration;
    use std::thread::yield_now;
    use std::time::Duration;
    use tavla::{any_voice, Speech, Voice};

    #[test]
    #[should_panic]
    fn machine_without_states() {
        Machine::new(
            Sensors::builder().build(),
            Actuators::new(&None, &[]).unwrap(),
            &[],
        );
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_end_transition_target() {
        let mut machine = Machine::new(
            Sensors::builder().build(),
            Actuators::new(&None, &[]).unwrap(),
            &[State::builder()
                .name("with illegal end transition target")
                .end(1)
                .build()],
        );

        machine.update();
    }

    #[test]
    fn timeout_starts_after_ringing() {
        crate::log::init_test_logging();

        let ring_time = Duration::from_millis(200);
        let timeout = Duration::from_millis(350);
        let expected_duration = ring_time + timeout;

        let states = &[
            State::builder()
                .name("ringing")
                .ring_for(ring_time)
                .timeout(timeout, 1)
                .build(),
            State::builder().name("done").terminal(true).build(),
        ];

        let test_duration = time_until_done_when_no_input(states);

        assert_duration("execution time", expected_duration, test_duration);
    }

    #[test]
    fn timeout_starts_after_speech() {
        let text = ".........";
        let speech_time = actual_speech_time(text);
        let timeout = Duration::from_millis(220);
        let expected_duration = speech_time + timeout;

        dbg!(speech_time);

        let test_duration = time_until_done_when_no_input(&[
            State::builder()
                .name("speaking")
                .speech(text)
                .timeout(timeout, 1)
                .build(),
            State::builder().name("done").terminal(true).build(),
        ]);

        assert_duration("execution time", expected_duration, test_duration);
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

    fn time_until_done_when_no_input(states: &[State]) -> Duration {
        let test_start = Instant::now();

        let mut machine = Machine::new(
            Sensors::builder().build(),
            Actuators::new(&None, &[]).unwrap(),
            states,
        );

        while machine.update() {
            yield_now()
        } // Run until finished
        test_start.elapsed() // And report how long it took
    }
}
