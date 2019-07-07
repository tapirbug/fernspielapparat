use super::PlayerContext;
use crate::acts::Act;
use crate::acts::{Sound, SoundSpec};
use crate::err::compound_result;
use failure::Error;

/// Responsible for playing back multiple sounds at the same time
/// and transitioning between them.
pub struct Ensemble {
    /// Shared resources of the sounds.
    _player_ctx: PlayerContext,
    /// The spec that was used to create the sounds
    /// in the sound vector.
    ///
    /// Index is also its unique ID.
    /// Indexes/IDs are paired with the sounds vector.
    specs: Vec<SoundSpec>,
    /// A player for every possible sound.
    ///
    /// Index is also its unique ID.
    /// Indexes/IDs are paired with the specs vector.
    sounds: Vec<Sound>,
}

impl Ensemble {
    pub fn from_specs<'a, I: IntoIterator<Item = &'a SoundSpec>>(sounds: I) -> Result<Self, Error> {
        let specs = sounds.into_iter().cloned().collect::<Vec<SoundSpec>>();
        let ctx = PlayerContext::new()?;

        specs
            .iter()
            .map(|s| Sound::from_spec_with_ctx(s, &ctx))
            .collect::<Result<Vec<_>, Error>>()
            .map(|sounds| Ensemble {
                _player_ctx: ctx,
                specs,
                sounds,
            })
    }

    /// Activates all sounds at the given indexes and cancels all
    /// others.
    ///
    /// The indexes originate from the insertion order using the iterator
    /// passed to `from_specs`.
    pub fn transition_to(&mut self, target_sound_ids: &[usize]) -> Result<(), Error> {
        compound_result(self.sounds.iter_mut().enumerate().map(|(id, sound)| {
            if target_sound_ids.contains(&id) {
                // Activate sound or keep it active if in the target set
                sound.activate()
            } else {
                // Cancel sounds that are not in the new set or keep them cancelled
                sound.cancel()
            }
        }))
    }

    pub fn update(&mut self) -> Result<(), Error> {
        compound_result(self.sounds.iter_mut().map(|s| (*s).update()))
    }

    /// Checks if all non-loop sounds are done.
    pub fn non_loop_sounds_idle(&self) -> bool {
        self.sounds
            .iter()
            .zip(self.specs.iter())
            .all(|(sound, spec)| {
                spec.is_loop() ||
                    // Consider sounds that cannot currently be checked non-idle
                    sound.done().unwrap_or(false)
            })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn nothing_enabled_initially() {
        // given
        let specs = [
            SoundSpec::builder()
                .source(crate::testutil::TEST_MUSIC)
                .build(),
            SoundSpec::builder()
                .source(crate::testutil::TEST_MUSIC)
                .build(),
        ];
        let ensemble = Ensemble::from_specs(&specs).unwrap();

        // when
        let sounds_enabled_initially = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_initially = [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];

        // then
        assert!(
            sounds_enabled_initially == [false, false],
            "Expected nothing to be enabled before the first transition \
             Actually: {:?}",
            sounds_enabled_initially
        );
        assert!(
            sounds_playing_initially == [false, false],
            "Expected nothing to be enabled before the first transition \
             Actually: {:?}",
            sounds_enabled_initially
        );
    }

    #[test]
    fn alternating_states() {
        // given
        let state_1_ids = &[0];
        let state_2_ids = &[1];
        let specs = [
            SoundSpec::builder()
                .source(crate::testutil::TEST_MUSIC)
                .build(),
            SoundSpec::builder()
                .source(crate::testutil::TEST_MUSIC)
                .build(),
        ];
        let mut ensemble = Ensemble::from_specs(&specs).unwrap();

        // when
        ensemble.update().unwrap();
        ensemble.transition_to(state_1_ids).unwrap();
        ensemble.update().unwrap();

        let sounds_enabled_state1 = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state1 = [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];

        ensemble.update().unwrap();
        ensemble.transition_to(state_2_ids).unwrap();
        ensemble.update().unwrap();

        let sounds_enabled_state2 = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state2 = [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];

        ensemble.update().unwrap();
        ensemble.transition_to(state_1_ids).unwrap();
        ensemble.update().unwrap();

        let sounds_enabled_state1_again = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state1_again =
            [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];

        ensemble.update().unwrap();
        ensemble.transition_to(state_2_ids).unwrap();
        ensemble.update().unwrap();

        let sounds_enabled_state2_again = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state2_again =
            [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];

        // then
        assert!(
            [sounds_enabled_state1, sounds_playing_state1] == [[true, false], [true, false]],
            "Expected only first sound to be enabled and playing in state 1. \
             Actually: {:?}",
            [sounds_enabled_state1, sounds_playing_state1]
        );
        assert!(
            [sounds_enabled_state2, sounds_playing_state2] == [[false, true], [false, true]],
            "Expected only second sound to be enabled and playing in state 2. \
             Actually: {:?}",
            [sounds_enabled_state2, sounds_playing_state2]
        );
        assert!(
            [sounds_enabled_state1_again, sounds_playing_state1_again]
                == [[true, false], [true, false]],
            "Expected only first sound to be enabled and playing in state 1. \
             Even when doing the same thing over again. \
             Actually: {:?}",
            [sounds_enabled_state1_again, sounds_playing_state1_again]
        );
        assert!(
            [sounds_enabled_state2_again, sounds_playing_state2_again]
                == [[false, true], [false, true]],
            "Expected only second sound to be enabled and playing in state 2. \
             Even when doing the same thing over again. \
             Actually: {:?}",
            [sounds_enabled_state2_again, sounds_playing_state2_again]
        );
    }

    #[test]
    fn continuing_sound() {
        // given
        let state_1_ids = &[0, 1];
        let state_2_ids = &[0];
        let specs = [
            SoundSpec::builder()
                .source(crate::testutil::TEST_MUSIC)
                .build(),
            SoundSpec::builder()
                .source(crate::testutil::TEST_MUSIC)
                .build(),
        ];
        let mut ensemble = Ensemble::from_specs(&specs).unwrap();

        /// Time to wait between transitions
        const TIME_BETWEEN: Duration = Duration::from_millis(500);

        // when
        ensemble.transition_to(state_1_ids).unwrap();
        let sounds_enabled_state1 = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state1 = [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];
        let sound_positions_t_1 = [ensemble.sounds[0].played(), ensemble.sounds[1].played()];

        sleep(TIME_BETWEEN);

        ensemble.transition_to(state_2_ids).unwrap();
        let sounds_enabled_state2 = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state2 = [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];
        let sound_positions_t_2 = [ensemble.sounds[0].played(), ensemble.sounds[1].played()];

        sleep(TIME_BETWEEN);

        ensemble.transition_to(state_1_ids).unwrap();
        let sounds_enabled_state1_again = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state1_again =
            [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];
        let sound_positions_t_3 = [ensemble.sounds[0].played(), ensemble.sounds[1].played()];

        sleep(TIME_BETWEEN);

        ensemble.transition_to(state_2_ids).unwrap();
        let sounds_enabled_state2_again = [
            !ensemble.sounds[0].done().unwrap(),
            !ensemble.sounds[1].done().unwrap(),
        ];
        let sounds_playing_state2_again =
            [ensemble.sounds[0].playing(), ensemble.sounds[1].playing()];
        let sound_positions_t_4 = [ensemble.sounds[0].played(), ensemble.sounds[1].played()];

        // then
        assert!(
            [sounds_enabled_state1, sounds_playing_state1] == [[true, true], [true, true]],
            "Expected both sounds to be enabled and playing in state 1. \
             Actually: {:?}",
            [sounds_enabled_state1, sounds_playing_state1]
        );
        assert!(
            [sounds_enabled_state2, sounds_playing_state2] == [[true, false], [true, false]],
            "Expected only first sound to be enabled and playing in state 2. \
             Actually: {:?}",
            [sounds_enabled_state2, sounds_playing_state2]
        );
        assert!(
            [sounds_enabled_state1_again, sounds_playing_state1_again]
                == [[true, true], [true, true]],
            "Expected both sounds to be enabled and playing in state 1. \
             Even when doing the same thing over again. \
             Actually: {:?}",
            [sounds_enabled_state1_again, sounds_playing_state1_again]
        );
        assert!(
            [sounds_enabled_state2_again, sounds_playing_state2_again]
                == [[true, false], [true, false]],
            "Expected only first sound to be enabled and playing in state 2. \
             Even when doing the same thing over again. \
             Actually: {:?}",
            [sounds_enabled_state2_again, sounds_playing_state2_again]
        );

        assert!(
            almost_equal(sound_positions_t_1[0], Duration::from_millis(0))
                && almost_equal(sound_positions_t_1[1], Duration::from_millis(0)),
            "Expected playback of both sounds near the start at the beginning of state 1. \
            \nSound 1 position: {:?}
            \nSound 2 position: {:?}",
            sound_positions_t_1[0],
            sound_positions_t_1[1]
        );
        assert!(
            almost_equal(sound_positions_t_2[0], TIME_BETWEEN) &&
                almost_equal(sound_positions_t_2[1], TIME_BETWEEN),
            "Expected both sounds to have made the same progress when state1 just finished the first time \
            and state 2 just started. \
            \nSound 1 position: {:?}, Expected: {:?}
            \nSound 2 position: {:?}, Expected: {:?}",
            sound_positions_t_2[0], TIME_BETWEEN,
            sound_positions_t_2[1], Duration::from_millis(0)
        );
        assert!(
            almost_equal(sound_positions_t_3[0], 2 * TIME_BETWEEN)
                && almost_equal(sound_positions_t_3[1], Duration::from_millis(0)),
            "Expected sound 1 to have kept playing and sound 2 to be rewinded
            when re-entering state 1. \
            \nSound 1 position: {:?}, Expected: {:?}
            \nSound 2 position: {:?}, Expected: {:?}",
            sound_positions_t_3[0],
            2 * TIME_BETWEEN,
            sound_positions_t_3[1],
            Duration::from_millis(0)
        );
        assert!(
            almost_equal(sound_positions_t_4[0], 3 * TIME_BETWEEN)
                && almost_equal(sound_positions_t_4[1], TIME_BETWEEN),
            "Expected playback of sound 1 to be ahead when re-entering state 2. \
            \nSound 1 position: {:?}, Expected: {:?}
            \nSound 2 position: {:?}, Expected: {:?}",
            sound_positions_t_4[0],
            3 * TIME_BETWEEN,
            sound_positions_t_4[1],
            TIME_BETWEEN
        );
    }

    #[test]
    fn not_idle_after_reenter_finished() {
        // given
        let specs = [SoundSpec::builder()
            .source(crate::testutil::TEST_MUSIC)
            .build()];
        let mut ensemble = Ensemble::from_specs(&specs).expect("could not make ensemble");

        // when
        let initially_idle = ensemble.non_loop_sounds_idle();

        ensemble.transition_to(&[0]).unwrap();
        ensemble.update().unwrap();

        let idle_after_enter = ensemble.non_loop_sounds_idle();

        ensemble.sounds[0].fast_forward(Duration::from_millis(200));
        ensemble.update().unwrap();

        let idle_after_ff = ensemble.non_loop_sounds_idle();

        sleep(Duration::from_millis(500));
        ensemble.update().unwrap();

        let idle_after_finish = ensemble.non_loop_sounds_idle();

        ensemble.update().unwrap();
        ensemble.transition_to(&[]).unwrap(); // leave
        ensemble.update().unwrap();
        ensemble.transition_to(&[0]).unwrap(); // re-enter
        ensemble.update().unwrap();

        let idle_after_reenter = ensemble.non_loop_sounds_idle();
        sleep(Duration::from_millis(50));
        ensemble.update().unwrap();
        let idle_after_reenter_and_wait = ensemble.non_loop_sounds_idle();

        // then
        assert!(initially_idle);
        assert!(!idle_after_enter);
        assert!(!idle_after_ff);
        assert!(idle_after_finish);
        assert!(!idle_after_reenter);
        assert!(!idle_after_reenter_and_wait);
    }

    fn delta(duration1: Duration, duration2: Duration) -> Duration {
        if duration1 > duration2 {
            duration1 - duration2
        } else {
            duration2 - duration1
        }
    }

    fn almost_equal(duration1: Duration, duration2: Duration) -> bool {
        // For better accuracy, we should wait that the new players
        // are ready when transitioning states.
        const EPS: Duration = Duration::from_millis(250);
        delta(duration1, duration2) <= EPS
    }
}
