use crate::acts::{Act, Ring, Sound, SoundSpec, Wait};
use crate::err::compound_result;
use crate::phone::Phone;
use crate::states::State;
use failure::Error;
use log::{debug, error, warn};
use std::mem::replace;
use std::sync::{Arc, Mutex, PoisonError};
use tavla::{any_voice, Voice};

pub struct Actuators {
    active: Vec<Box<dyn Act>>,
    sounds: Vec<Sound>,
    phone: Option<Arc<Mutex<Phone>>>,
    sound_specs: Vec<SoundSpec>,
}

impl Actuators {
    pub fn new(
        phone: &Option<Arc<Mutex<Phone>>>,
        sound_specs: &[SoundSpec],
    ) -> Result<Self, Error> {
        let sounds = sound_specs
            .iter()
            .map(Sound::from_spec)
            .collect::<Result<Vec<Sound>, _>>()?;

        let actuators = Actuators {
            active: vec![],
            sounds,
            phone: phone.as_ref().map(Arc::clone),
            sound_specs: sound_specs.to_vec(),
        };

        Ok(actuators)
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

        // update sounds
        for sound_act in self.sounds.iter_mut() {
            sound_act
                .update()
                .unwrap_or_else(|_| warn!("Failed to update sound: {:?}", &sound_act));
        }

        Ok(())
    }

    /// Returns `true` all acts are done or have been cancelled.
    ///
    /// Returns `false` if some actuators are still working, e.g.
    /// speech is still ongoing.
    pub fn done(&self) -> bool {
        self.active.is_empty()
            && self
                .sounds
                .iter()
                .zip(self.sound_specs.iter())
                .all(|(sound, spec)| spec.is_loop() || sound.done().unwrap_or(true))
    }

    pub fn transition_to(&mut self, state: &State) -> Result<(), Error> {
        self.transition_sounds(state)?;
        self.transition_content(self.make_act_states(state))?;
        Ok(())
    }

    fn transition_sounds(&mut self, state: &State) -> Result<(), Error> {
        for (idx, sound) in self.sounds.iter_mut().enumerate() {
            let done = sound.done().unwrap_or(true);

            if state.sounds().contains(&idx) {
                if done {
                    // Activate sounds in the set that are currently inactive
                    debug!("Starting sound: {:?}", self.sound_specs[idx].source());
                    sound
                        .activate()
                        .unwrap_or_else(|e| warn!("Failed to activate sound: {}", e));
                } else {
                    // And keep the ones that are already playing
                    debug!(
                        "Keeping active sound on re-enter: {:?}",
                        self.sound_specs[idx].source()
                    );
                }
            } else if !done {
                // Cancel sounds that are not in the new set
                debug!("Stopping sound: {:?}", self.sound_specs[idx].source());
                sound
                    .cancel()
                    .unwrap_or_else(|e| warn!("Failed to deactivate sound: {:?}", e));
            }
        }

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

    pub fn transition_content(&mut self, next_acts: Vec<Box<dyn Act>>) -> Result<(), Error> {
        if let Err(errs) = cancel_all(&mut replace(&mut self.active, next_acts)) {
            warn!("Some acts could not be cancelled: {}", errs);
        };
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
