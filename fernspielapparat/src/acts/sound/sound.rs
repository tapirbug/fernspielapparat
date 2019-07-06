use super::{Player, ReenterBehavior, SoundSpec};
use crate::acts::Act;
use derivative::Derivative;
use failure::Error;
use log::debug;
use std::cmp::max;
use std::time::Duration;

/// Plays a sound file in the background.
#[derive(Derivative)]
#[derivative(PartialEq, Eq, Hash, Debug)]
pub struct Sound {
    #[derivative(Hash = "ignore", PartialEq = "ignore", Debug = "ignore")]
    player: Player,
    spec: SoundSpec,
    /// If `true`, the last interaction with the sound from client code
    /// was `activate`, otherwise `cancel`. If neither has been called,
    /// is `false`.
    activated: bool,
    never_activated: bool,
}

impl Sound {
    pub fn from_spec(spec: &SoundSpec) -> Result<Self, Error> {
        let player = Player::new(spec.source())?;

        let sound = Self {
            player,
            spec: spec.clone(),
            activated: false,
            never_activated: true,
        };

        Ok(sound)
    }

    fn loop_or_deactivate_on_finish(&mut self) {
        if let Ok(false) = self.player.playing() {
            if self.spec.is_loop() && self.activated {
                // note: not applying the start offset on purpose for looping
                self.player.rewind();
            } else {
                self.activated = false;
            }
        }
    }

    fn seek_on_enter(&mut self, was_active: bool) {
        if was_active {
            // Activating while already active, keep playing
            debug!("Keeping sound that is already playing: {:?}", &self.spec);
        } else if self.never_activated {
            // Entering for the first time
            self.player.seek(self.spec.start_offset());
        } else {
            // Re-entering
            self.reenter();
        }

        self.never_activated = false;
    }

    fn reenter(&mut self) {
        debug!("Re-entering: {:?}", &self.spec);

        let looping = self.spec.is_loop();
        let reenter = self.spec.reenter_behavior();
        match reenter {
            // Do nothing when re-entering a loop without backoff
            ReenterBehavior::Rewind if looping => (),
            other => self.player.seek(match other {
                // Re-entering a non-loop with no backoff, rewind
                ReenterBehavior::Rewind => {
                    debug!("Rewinding: {:?}", &self.spec);
                    self.spec.start_offset()
                }
                // Backoff configured and looping too, wrap around the start to the end when backing off
                ReenterBehavior::Backoff(backoff) if looping => self.backoff_looping(backoff),
                // Backoff configured, do normal backoff
                ReenterBehavior::Backoff(backoff) => self.backoff_non_looping(backoff),
            }),
        }
    }

    /// backoff without looping
    /// subtract the backoff from the current playback
    /// position and clamp at the start offset.
    fn backoff_non_looping(&self, backoff: Duration) -> Duration {
        let played = self.player.played();
        let start_offset = self.spec.start_offset();

        let to = max(
            start_offset,
            played.checked_sub(backoff).unwrap_or(start_offset),
        );

        debug!(
            "Seeking for backoff ({backoff:?}) in non-looping sound, \
             played {played:?}, target: {target:?}, start offset: {offset:?}",
            backoff = backoff,
            played = played,
            target = to,
            offset = start_offset
        );

        to
    }

    /// backoff in looping
    /// subtract the backoff from the current playback
    /// position and wrap around the end when reaching zero
    fn backoff_looping(&self, backoff: Duration) -> Duration {
        let duration = self.player.duration();
        let played = self.player.played();
        let backoff = duration_mod(backoff, duration);

        let to = if backoff > played {
            // wrap around the end
            duration - (backoff - played)
        } else {
            played - backoff
        };

        debug!("Seeking for backoff in looping sound, target: {:?}", to);

        to
    }

    /// Allows tests in other modules to check if the player is actually playing.
    ///
    /// Use `done` for real code.
    #[cfg(test)]
    pub fn playing(&self) -> bool {
        self.player.playing().unwrap()
    }

    /// Allows tests in other modules to check playback position.
    ///
    /// Do not use in real code.
    #[cfg(test)]
    pub fn played(&self) -> Duration {
        self.player.played()
    }

    #[cfg(test)]
    pub fn fast_forward(&mut self, to_before_finish: Duration) {
        self.player.seek(self.player.duration() - to_before_finish);
    }
}

fn duration_mod(duration: Duration, max_duration: Duration) -> Duration {
    Duration::from_nanos((duration.as_nanos() as u64) % (max_duration.as_nanos() as u64))
}

impl Act for Sound {
    fn activate(&mut self) -> Result<(), Error> {
        let was_active = self.activated;
        self.activated = true;
        self.seek_on_enter(was_active);
        self.player.play()?; // Need to start playing first to make seeking possible
        Ok(())
    }

    fn update(&mut self) -> Result<(), Error> {
        self.loop_or_deactivate_on_finish();
        Ok(())
    }

    fn done(&self) -> Result<bool, Error> {
        Ok(!self.activated)
    }

    fn cancel(&mut self) -> Result<(), Error> {
        self.activated = false;
        self.player.pause()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::sleep;
    use std::time::Instant;

    #[test]
    fn rewind_on_reenter_when_fully_played() {
        crate::testutil::enable_logging();

        // given
        let mut sound = Sound::from_spec(
            &SoundSpec::builder()
                .source("test/A Good Bass for Gambling.mp3")
                .build(),
        )
        .expect("Could not make sound");
        let duration = sound.player.duration();

        // when
        sound.activate().unwrap(); // start playing
                                   // seek to the end by directly accesing the player field
                                   // effect should be the same as when waiting for that time
        sound.player.seek(duration - Duration::from_secs(1));

        sound.update().unwrap();

        // Let the sound finish
        let done_after_seek = sound.done().unwrap();
        sleep(Duration::from_secs(2)); // Make sure track ended
        sound.update().unwrap();
        let done_after_seek_and_wait = sound.done().unwrap();

        // deactivate
        sound.cancel().unwrap();
        sound.update().unwrap();
        let pos_after_cancel = sound.player.played();

        // re-enabling should start over
        sound.activate().unwrap();
        sound.update().unwrap();
        let pos_after_reactivate = sound.player.played();

        // then
        assert!(
            !done_after_seek,
            "Expecting sound to not be done when seeking near the end"
        );
        assert!(
            done_after_seek_and_wait,
            "Expecting done after seeking near the end and then cancelling"
        );
        assert!(
            almost_equal(duration, pos_after_cancel),
            "Expecting player position to be near or at the end, after playing through. \
             Player position: {pos:?}, \
             Duration: {duration:?}",
            pos = pos_after_cancel,
            duration = duration
        );
        assert!(
            almost_equal(pos_after_reactivate, Duration::from_secs(0)),
            "Expected player to start over after reactivating. \
             Actual playback position: {:?}",
            pos_after_reactivate
        );
    }

    #[test]
    fn backoff_on_reenter_for_non_looping_clamp_at_start_offset() {
        crate::testutil::enable_logging();

        // given
        let mut sound = Sound::from_spec(
            &SoundSpec::builder()
                .source("test/A Good Bass for Gambling.mp3")
                .start_offset(10)
                .unwrap()
                .backoff(2)
                .unwrap()
                .build(),
        )
        .expect("Could not make sound");

        // when
        assert!(sound.done().unwrap(), "Sound should be initally done");
        sound.activate().unwrap();
        assert!(
            !sound.done().unwrap(),
            "Sound should not be done after activating but before first update"
        );
        sound.update().unwrap();
        assert!(
            !sound.done().unwrap(),
            "Sound should not be done after activating and after first update"
        );

        let played_before_cancel = sound.player.played();
        sound.cancel().unwrap();
        let played_after_cancel = sound.player.played();
        let done_after_cancel = sound.done().unwrap();

        sound.activate().unwrap();
        let played_after_reactivate = sound.player.played();

        sound.update().unwrap();
        let played_after_reactivate_and_update = sound.player.played();

        // then
        assert!(
            played_before_cancel >= Duration::from_secs(10),
            "Expecting player to be past start offset. \
             Actual position: {pos:?}. Start offset: {start:?}.",
            pos = played_before_cancel,
            start = Duration::from_secs(10)
        );
        assert!(
            played_after_cancel >= played_before_cancel,
            "Expecting player to still be past start offset after cancel"
        );
        assert!(done_after_cancel, "Sound should be not done after cancel");
        assert!(
            almost_equal(played_after_reactivate_and_update, played_before_cancel),
            "Expecting backoff to clamp at start offset for non-looping sounds, unsigned diff is {:?}",
            delta(played_after_reactivate_and_update, played_before_cancel)
        );
        assert!(
            almost_equal(played_after_reactivate, played_before_cancel),
            "Expecting backoff to clamp at start offset for non-looping sounds, unsigned diff is {:?}",
            delta(played_after_reactivate, played_before_cancel)
        );
    }

    #[test]
    fn backoff_on_reenter_for_non_looping() {
        crate::testutil::enable_logging();

        // given
        let mut sound = Sound::from_spec(
            &SoundSpec::builder()
                .source("test/A Good Bass for Gambling.mp3")
                .backoff(2)
                .unwrap()
                .build(),
        )
        .expect("Could not make sound");

        // when
        assert!(sound.done().unwrap(), "Sound should be initially done");
        sound.activate().unwrap();
        assert!(
            !sound.done().unwrap(),
            "Sound should not be done after activating but before first update"
        );
        sound.update().unwrap();
        assert!(
            !sound.done().unwrap(),
            "Sound should not be done after activating and after first update"
        );

        // Wait for some time so backoff does not clamp at start offset
        sleep(Duration::from_secs(3));

        let played_before_cancel = sound.player.played();
        sound.cancel().unwrap();
        let played_after_cancel = sound.player.played();
        let done_after_cancel = sound.done().unwrap();

        sound.activate().unwrap();
        let played_after_reactivate = sound.player.played();

        sound.update().unwrap();
        let played_after_reactivate_and_update = sound.player.played();

        // then
        assert!(
            played_after_cancel >= played_before_cancel,
            "Expecting player to still be past start offset after cancel"
        );
        assert!(done_after_cancel, "Sound should be not done after cancel");
        assert!(
            played_after_reactivate_and_update < played_before_cancel,
            "Expecting backoff to seek backwards after reactivate and update, unsigned diff is {:?}", delta(played_after_reactivate_and_update, played_before_cancel)
        );
        assert!(
            delta(played_after_reactivate_and_update, played_before_cancel)
                >= Duration::from_secs(2),
            "Expecting two seconds of backoff, unsigned diff is {:?}",
            delta(played_after_reactivate_and_update, played_before_cancel)
        );
        assert!(
            played_after_reactivate < played_before_cancel,
            "Expecting backoff to seek backwards after reactivate, unsigned diff is {:?}",
            delta(played_after_reactivate, played_before_cancel)
        );
        assert!(
            delta(played_after_reactivate, played_before_cancel) >= Duration::from_secs(2),
            "Expecting two seconds of backoff, unsigned diff is {:?}",
            delta(played_after_reactivate, played_before_cancel)
        );
    }

    #[test]
    fn backoff_in_loop_rollover() {
        // given
        let backoff_i: u64 = 5;
        let backoff = Duration::from_secs(backoff_i);
        let mut sound = Sound::from_spec(
            &SoundSpec::builder()
                .source("test/A Good Bass for Gambling.mp3")
                .start_offset(2)
                .unwrap()
                .backoff(backoff_i as f64)
                .unwrap()
                .looping(true)
                .build(),
        )
        .expect("Could not make sound");
        let duration = sound.player.duration();

        // when
        sound.activate().unwrap();
        sound.update().unwrap();
        let played_before_cancel = sound.player.played();
        sound.cancel().unwrap();
        sound.update().unwrap();
        sound.activate().unwrap();
        let played_after_reactivate = sound.player.played();
        sound.update().unwrap();
        let played_after_reactivate_and_update = sound.player.played();

        // then
        assert!(
            delta(played_after_reactivate, duration) <= Duration::from_millis(3000),
            "Expected rollover and the track being played near the end (from position 2, \
             backoff of five seconds, should be approx. three seconds before the end). \
             Duration: {duration:?}, \
             Backoff: {backoff:?}, \
             Playback position before backoff: {before_backoff:?} \
             Playback position after backoff: {after_backoff:?}",
            duration = duration,
            backoff = backoff,
            before_backoff = played_before_cancel,
            after_backoff = played_after_reactivate
        );
        assert!(
            duration > Duration::from_secs(10),
            "Expecting sufficiently long track"
        );
        assert!(
            played_before_cancel >= Duration::from_secs(2)
                && played_before_cancel <= Duration::from_secs(3),
            "Expecting start offset to work on first play in looping"
        );
        assert!(
            almost_equal(played_after_reactivate, played_after_reactivate_and_update),
            "Expecting played to not change much after update"
        );
    }

    #[test]
    fn once_with_offset() {
        let mut sound = Sound::from_spec(
            &SoundSpec::builder()
                .source("test/A Good Bass for Gambling.mp3")
                .start_offset(2.0 * 60.0 + 34.0)
                .unwrap() // Start almost at the end
                .build(),
        )
        .unwrap();

        sound.activate().unwrap();
        sound.update().unwrap();
        assert!(!sound.done().unwrap());
        let play_start_time = Instant::now();
        while !sound.done().unwrap() {
            sleep(Duration::from_secs(1));
            sound.update().unwrap();
        }
        assert!(play_start_time.elapsed() < Duration::from_secs(5));
        assert!(play_start_time.elapsed() > Duration::from_millis(50))
    }

    #[test]
    fn elevator_music_loop_then_cancel() {
        let mut sound = Sound::from_spec(
            &SoundSpec::builder()
                .source("test/A Good Bass for Gambling.mp3")
                .start_offset(2 * 60 + 30)
                .unwrap()
                .looping(true)
                .build(),
        )
        .expect("Could not make sound");

        sound.activate().unwrap();
        sleep(Duration::from_millis(2));
        sound.update().unwrap();
        assert!(!sound.done().unwrap());
        sleep(Duration::from_millis(4_000));
        sound.update().unwrap();
        assert!(!sound.done().unwrap());

        sound.cancel().unwrap();

        assert!(sound.done().unwrap());
    }

    fn delta(duration1: Duration, duration2: Duration) -> Duration {
        if duration1 > duration2 {
            duration1 - duration2
        } else {
            duration2 - duration1
        }
    }

    fn almost_equal(duration1: Duration, duration2: Duration) -> bool {
        const EPS: Duration = Duration::from_millis(200);
        delta(duration1, duration2) <= EPS
    }
}
