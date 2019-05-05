use crate::acts::{Act, Ring, Sound, Wait};
use crate::err::compound_result;
use crate::phone::Phone;
use crate::states::State;
use failure::Error;
use log::{debug, error, warn};
use std::collections::HashSet;
use std::mem::replace;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use tavla::{any_voice, Voice};

pub struct Actuators {
    active: Vec<Box<dyn Act>>,
    active_environment: Vec<Sound>,
    phone: Option<Arc<Mutex<Phone>>>,
}

impl Actuators {
    pub fn new(phone: &Option<Arc<Mutex<Phone>>>) -> Self {
        Actuators {
            active: vec![],
            active_environment: vec![],
            phone: phone.as_ref().map(Arc::clone),
        }
    }

    pub fn update(&mut self) -> Result<(), Error> {
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

        Ok(())
    }

    /// Returns `true` all acts are done or have been cancelled.
    ///
    /// Returns `false` if some actuators are still working, e.g.
    /// speech is still ongoing.
    pub fn done(&self) -> bool {
        self.active.is_empty()
    }

    pub fn transition_to(&mut self, state: &State) -> Result<(), Error> {
        self.transition_content(self.make_act_states(state))?;
        self.transition_environment(state.environment().iter().cloned().collect())?;
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

        state
            .content()
            .iter()
            .map(Sound::new)
            .for_each(|s| acts.push(Box::new(s)));

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

    pub fn transition_content(&mut self, next_acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
        if let Err(errs) = cancel_all(&mut replace(&mut self.active, next_acts)) {
            warn!("Some acts could not be cancelled: {}", errs);
        };
        Ok(())
    }

    pub fn transition_environment(
        &mut self,
        next_environment: HashSet<PathBuf>,
    ) -> Result<(), Error> {
        // keep union of old and new
        let (keeping_sounds, obsolete_sounds): (HashSet<_>, HashSet<_>) =
            replace(&mut self.active_environment, vec![])
                .into_iter()
                .partition(|old| next_environment.contains(&PathBuf::from(old.source())));

        compound_result(obsolete_sounds.into_iter().map(|mut a| a.cancel()))?;

        let keeping: HashSet<PathBuf> = keeping_sounds
            .iter()
            .map(|k| PathBuf::from(k.source()))
            .collect();

        for potentially_new in next_environment.into_iter() {
            if !keeping.contains(&potentially_new) {
                self.active_environment.push(Sound::new(potentially_new))
            }
        }

        self.active_environment.extend(keeping_sounds);

        Ok(())
    }
}

fn cancel_all(acts: &mut Vec<Box<dyn Act>>) -> Result<(), Error> {
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
