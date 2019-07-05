use crate::acts::Act;
use derivative::Derivative;
use failure::Error;
use log::debug;
use play::Player;
use std::path::{Path, PathBuf};
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
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct SoundSpec {
    source: PathBuf,
    end: EndBehavior,
    start_offset: Duration,
    backoff: Duration,
}

impl SoundSpec {
    fn new(source: PathBuf, end: EndBehavior, start_offset: Duration, backoff: Duration) -> Self {
        SoundSpec {
            source,
            end,
            start_offset,
            backoff,
        }
    }

    pub fn once<P: AsRef<Path>>(source: P, start_offset: Duration, backoff: Duration) -> Self {
        Self::new(
            source.as_ref().into(),
            EndBehavior::Done,
            start_offset,
            backoff,
        )
    }

    pub fn repeat<P: AsRef<Path>>(source: P, start_offset: Duration, backoff: Duration) -> Self {
        Self::new(
            source.as_ref().into(),
            EndBehavior::Loop,
            start_offset,
            backoff,
        )
    }

    #[cfg(test)]
    pub fn seek_then_repeat<P: AsRef<Path>>(source: P, start_offset: Duration) -> Self {
        Self::new(
            source.as_ref().into(),
            EndBehavior::Loop,
            start_offset,
            Duration::from_millis(0),
        )
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn is_loop(&self) -> bool {
        if let EndBehavior::Loop = self.end {
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum EndBehavior {
    Done,
    Loop,
}

impl Sound {
    pub fn from_spec(spec: &SoundSpec) -> Result<Self, Error> {
        let player = Player::new(spec.source())?;

        let sound = Self {
            player,
            spec: spec.clone(),
            activated: false,
        };

        Ok(sound)
    }

    fn rewind(&mut self) {
        self.player.seek(Duration::from_millis(0));
    }

    fn ensure_loop_if_active(&mut self) -> Result<(), Error> {
        if self.activated && self.spec.is_loop() && !self.player.playing() {
            self.rewind();
            self.player.play()?;
        }
        Ok(())
    }

    fn rewind_unless_already_active(&mut self) {
        if !self.activated {
            debug!("Rewinding sound on re-enter: {:?}", &self.spec);
            self.rewind()
        }
    }

    fn seek_on_reenter(&mut self, was_active: bool) {
        if was_active {
            // Was previously playing, keep going
        } else {
            let has_backoff = self.spec.backoff.as_millis() > 0;
            self.player.seek(if has_backoff {
                // Non-zero backoff configured, instead of rewinding
                // subtract the backoff from the current playback
                // position and clamp at the start of the file.
                self.player
                    .played()
                    .checked_sub(self.spec.backoff)
                    .unwrap_or_else(|| Duration::from_millis(0))
            } else {
                // No backoff configured, rewind to the start offset
                self.spec.start_offset
            });
        }
    }
}

impl Act for Sound {
    fn activate(&mut self) -> Result<(), Error> {
        let was_active = self.activated;
        self.activated = true;
        self.rewind_unless_already_active();
        self.player.play()?;
        self.seek_on_reenter(was_active);
        Ok(())
    }

    fn update(&mut self) -> Result<(), Error> {
        self.ensure_loop_if_active()
    }

    fn done(&self) -> Result<bool, Error> {
        Ok(!self.player.playing())
    }

    fn cancel(&mut self) -> Result<(), Error> {
        self.activated = false;
        self.player.pause()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Instant;

    use std::thread::sleep;
    #[test]
    fn once_with_offset() {
        let mut sound = Sound::from_spec(&SoundSpec::once(
            "test/A Good Bass for Gambling.mp3",
            Duration::from_secs(2 * 60 + 34), // Start almost at the end
            Duration::from_millis(0),         // No backoff
        ))
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
        let mut sound = Sound::from_spec(&SoundSpec::seek_then_repeat(
            "test/A Good Bass for Gambling.mp3",
            Duration::from_secs(2 * 60 + 30),
        ))
        .expect("Could not make sound");

        sound.activate().unwrap();
        sound.update().unwrap();
        assert!(!sound.done().unwrap());
        sleep(Duration::from_millis(4_000));
        sound.update().unwrap();
        assert!(!sound.done().unwrap());

        sound.cancel().unwrap();

        assert!(sound.done().unwrap());
    }
}

mod play {
    use failure::{bail, format_err, Error};
    use std::cmp;
    use std::convert::TryInto;
    use std::path::Path;
    use std::sync::mpsc::channel;
    use std::time::Duration;
    use std::time::Instant;
    use vlc::{self, Instance, Media, MediaPlayer, State};

    const READ_DURATION_TIMEOUT: Duration = Duration::from_secs(4);
    const PAUSE_DIRTY_TIMEOUT: Duration = Duration::from_millis(50);

    pub struct Player {
        _instance: Instance,
        media: Media,
        player: MediaPlayer,
        duration: Duration,
        /// There is some lag between pausing the player and when its state
        /// has changed to paused. We keep track ourselves of whether or not
        /// the player is paused and use the real media state after some timeout
        /// `PAUSE_DIRTY_TIMEOUT`.
        last_pause_request: Option<(Instant, bool)>,
    }

    impl Player {
        pub fn new(file: impl AsRef<Path>) -> Result<Self, Error> {
            let instance = Instance::new().ok_or_else(|| format_err!("Could not load libvlc"))?;

            let media = Media::new_path(&instance, file.as_ref())
                .ok_or_else(|| format_err!("Could not load media {:?}", file.as_ref()))?;

            let player = MediaPlayer::new(&instance)
                .ok_or_else(|| format_err!("Could not load media {:?}", file.as_ref()))?;

            let (tx, rx) = channel::<Duration>();
            media
                .event_manager()
                .attach(vlc::EventType::MediaDurationChanged, move |e, _| match e {
                    vlc::Event::MediaDurationChanged(duration) => {
                        tx.send(Duration::from_millis(duration.try_into().unwrap_or(0)))
                            .ok();
                    }
                    _ => (),
                })
                .map_err(|_| format_err!("Could not obtain media duration: {:?}", file.as_ref()))?;

            media.parse();
            player.set_media(&media);

            let duration = rx
                .recv_timeout(READ_DURATION_TIMEOUT)
                .map_err(|_| format_err!("Could not obtain media duration: {:?}", file.as_ref()))?;

            player.pause();

            Ok(Player {
                _instance: instance,
                media,
                player,
                duration,
                last_pause_request: None,
            })
        }

        pub fn play(&mut self) -> Result<(), Error> {
            if self.playing() {
                return Ok(());
            }

            self.player.play().map_err(|_| {
                format_err!(
                    "Could not play media {:?}",
                    self.media.mrl().unwrap_or("<Could not obtain mrl>".into())
                )
            })?;
            self.last_pause_request = Some((Instant::now(), false));

            Ok(())
        }

        pub fn pause(&mut self) -> Result<(), Error> {
            if !self.playing() {
                return Ok(());
            }

            if !self.player.can_pause() {
                bail!(
                    "Media can not currently be paused {:?}",
                    self.media.mrl().unwrap_or("<Could not obtain mrl>".into())
                );
            }

            self.player.set_pause(true);
            self.last_pause_request = Some((Instant::now(), true));

            // note: this does not hold right away, VLC needs some time
            // assert_eq!(self.player.state(), State::Paused);

            Ok(())
        }

        pub fn playing(&self) -> bool {
            match self.last_pause_request {
                Some((at, paused)) if at.elapsed() < PAUSE_DIRTY_TIMEOUT => !paused,
                _ => match self.player.state() {
                    State::Opening | State::Buffering | State::Playing => true,
                    State::NothingSpecial
                    | State::Paused
                    | State::Stopped
                    | State::Ended
                    | State::Error => false,
                },
            }
        }

        pub fn played(&self) -> Duration {
            Duration::from_millis(self.player.get_time().unwrap_or(0).try_into().unwrap_or(0))
        }

        /// Full duration of the played media.
        pub fn duration(&self) -> Duration {
            self.duration
        }

        pub fn seek(&mut self, from_start: Duration) {
            let from_start = cmp::min(self.duration(), from_start); // Skip to end if out of bounds
            self.player.set_time(
                from_start
                    .as_millis()
                    .try_into()
                    .expect("Duration was out of bounds"),
            );
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use std::thread::sleep;
        use std::time::{Duration, Instant};

        #[test]
        fn elevator_music() {
            let mut player =
                Player::new("test/A Good Bass for Gambling.mp3").expect("Could not make player");
            let play_start_time = Instant::now();
            player.play().expect("Could not play");
            assert!(player.playing());

            while player.playing() && play_start_time.elapsed() < Duration::from_secs(1) {
                sleep(Duration::from_secs(1))
            }
            assert!(play_start_time.elapsed() > Duration::from_secs(1));

            player.pause().unwrap();
            assert!(!player.playing());
            sleep(Duration::from_millis(500));

            player.play().unwrap();
            assert!(player.playing());
            sleep(PAUSE_DIRTY_TIMEOUT);
            assert!(player.playing());

            player.seek(player.duration() - Duration::from_millis(10));
            assert!(player.played() > Duration::from_secs(100));
            sleep(Duration::from_millis(15));
            assert!(
                !player.playing(),
                "Player should be paused after reaching the end of the media"
            );
        }
    }
}
