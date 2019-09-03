use crate::acts::{Act, Ensemble, Ring, SoundSpec, Wait};
use crate::err::compound_result;
use crate::evt::{Event, Responder, ResponderState};
use crate::phone::Phone;
use crate::result::Result;
use crate::states::State;
use log::{debug, error, warn};
use std::mem::replace;
use std::sync::{Arc, Mutex, PoisonError};
use tavla::{any_voice, Voice};

pub struct Actuators {
    active: Vec<Box<dyn Act>>,
    phone: Option<Arc<Mutex<Phone>>>,
    ensemble: Ensemble,
}

impl Actuators {
    pub fn new(phone: &Option<Arc<Mutex<Phone>>>, sound_specs: &[SoundSpec]) -> Result<Self> {
        let actuators = Actuators {
            active: vec![],
            ensemble: Ensemble::from_specs(sound_specs)?,
            phone: phone.as_ref().map(Arc::clone),
        };

        Ok(actuators)
    }

    /// Sets all actuators back into the initial state.
    pub fn reset(&mut self) -> Result<()> {
        self.ensemble.reset()
    }

    fn do_update(&mut self) -> Result<()> {
        // First give every act a chance to update
        let update_errs: Vec<_> = self
            .active
            .iter_mut()
            .map(|a| (*a).update())
            .filter_map(Result::err)
            .collect();

        if !update_errs.is_empty() {
            error!("Actuator update failures: {:?}", update_errs);
        }

        // remove finished acts
        self.active.retain(|a| {
            let done = a.done().unwrap_or(false);
            !done
        });

        // update sounds
        let ensemble_update = self.ensemble.update();
        if let Err(err) = ensemble_update {
            error!("Sound update failures: {:?}", err);
        }

        Ok(())
    }

    /// Returns `true` all acts are done or have been cancelled.
    ///
    /// Returns `false` if some actuators are still working, e.g.
    /// speech is still ongoing.
    pub fn done(&self) -> bool {
        self.active.is_empty() && self.ensemble.non_loop_sounds_idle()
    }

    pub fn transition_to(&mut self, state: &State) -> Result<()> {
        self.ensemble.transition_to(state.sounds())?;
        self.transition_content(self.make_act_states(state))?;
        Ok(())
    }

    fn make_act_states(&self, state: &State) -> Vec<Box<dyn Act>> {
        let mut acts: Vec<Box<dyn Act>> = vec![];

        if !state.speech().is_empty() {
            acts.push(Box::new(
                any_voice()
                    .expect("Could not load a voice")
                    .speak(state.speech())
                    .expect("Could not start speech for state"),
            ));
        }

        if let Some(duration) = state.ring_time() {
            if let Some(phone) = self.phone.as_ref() {
                acts.push(Box::new(
                    Ring::new(phone, duration).expect("Failed to start ring"),
                ))
            } else {
                // If no real bell available, do a silent bell for timeout purposes only
                acts.push(Box::new(Wait::new(duration)))
            }
        }

        acts
    }

    fn transition_content(&mut self, next_acts: Vec<Box<dyn Act>>) -> Result<()> {
        // replace self.active with new
        if let Err(errs) = cancel_all(&mut replace(&mut self.active, next_acts)) {
            warn!("Some acts could not be cancelled: {}", errs);
        };

        // and activate replaced contents
        compound_result(self.active.iter_mut().map(|a| (*a).activate()))
    }
}

fn cancel_all(acts: &mut Vec<Box<dyn Act>>) -> Result<()> {
    compound_result(acts.iter_mut().map(|a| (*a).cancel()))
}

impl Drop for Actuators {
    fn drop(&mut self) {
        let mut acts = &mut replace(&mut self.active, vec![]);

        match cancel_all(&mut acts) {
            Ok(_) => debug!("Actuators dropped at shutdown"),
            Err(e) => warn!("Failed to stop actuators at shutdown: {}", e),
        }

        if let Some(phone) = self.phone.take() {
            phone
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .unring()
                .unwrap_or_else(|e| warn!("Failed to unring phone at shutdown: {}", e));
        }
    }
}

impl Responder<State> for Actuators {
    fn respond(&mut self, event: &Event<State>) -> Result<()> {
        match event {
            Event::Start { initial } => {
                self.reset()?;
                self.transition_to(initial)
            }
            Event::Transition { to, .. } => self.transition_to(to),
            // don't care about non-transition events
            _ => Ok(()),
        }
    }

    fn update(&mut self) -> Result<ResponderState> {
        self.do_update()?;
        Ok(if self.done() {
            ResponderState::Idle
        } else {
            ResponderState::Running
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::testutil::{assert_duration, MediaInfo, WILHELM_SCREAM};
    use std::thread::yield_now;
    use std::time::{Duration, Instant};

    #[test]
    fn responder_state_changes_to_idle_when_ring_finished() {
        // given
        crate::log::init_test_logging();
        let mut actuators = Actuators::new(&None, &[]).expect("could not create actuators");
        let ring_duration = Duration::from_millis(300);
        let timeout_state = &State::builder().ring_for(ring_duration).build();
        let start_at_timeout_state = Event::Start {
            initial: timeout_state,
        };

        // when
        let time_before = Instant::now();

        actuators
            .respond(&start_at_timeout_state)
            .expect("failed to respond");

        let state_initial = actuators.update().unwrap();
        while let ResponderState::Running = actuators.update().unwrap() {
            yield_now();
        }
        let state_after = actuators.update().unwrap();

        let time_after = Instant::now();

        // then
        let actual_duration = time_after.duration_since(time_before);
        assert_duration("ring duration", ring_duration, actual_duration);
        assert_eq!(state_initial, ResponderState::Running);
        assert_eq!(state_after, ResponderState::Idle);
    }

    #[test]
    fn responder_state_changes_to_idle_when_non_loop_music_finished() {
        // given
        crate::log::init_test_logging();
        let mut actuators = Actuators::new(
            &None,
            &[SoundSpec::builder().source(WILHELM_SCREAM).build()],
        )
        .expect("could not create actuators");
        let timeout_state = &State::builder().sounds(vec![0]).build();
        let start_at_timeout_state = Event::Start {
            initial: timeout_state,
        };
        let expected_duration = MediaInfo::obtain(WILHELM_SCREAM).unwrap().actual_duration();

        // when
        let time_before = Instant::now();

        actuators
            .respond(&start_at_timeout_state)
            .expect("failed to respond");

        let state_initial = actuators.update().unwrap();
        while let ResponderState::Running = actuators.update().unwrap() {
            yield_now();
        }
        let state_after = actuators.update().unwrap();

        let time_after = Instant::now();

        // then
        let actual_duration = time_after.duration_since(time_before);
        assert_duration("scream duration", expected_duration, actual_duration);
        assert_eq!(state_initial, ResponderState::Running);
        assert_eq!(state_after, ResponderState::Idle);
    }
}
