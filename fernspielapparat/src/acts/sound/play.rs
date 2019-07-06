use failure::{bail, format_err, Error};
use log::warn;
use std::cmp::min;
use std::convert::TryInto;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::time::Instant;
use vlc::{self, Instance, Media, MediaPlayer, State};

const READ_DURATION_TIMEOUT: Duration = Duration::from_secs(4);
const PAUSE_DIRTY_TIMEOUT: Duration = Duration::from_millis(50);

/// Responsible for playback of a single file.
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
    /// When trying to seek but the media is paused, caching it here.
    /// This also happens upon construction, seeking the start.
    pending_seek: Option<Duration>,
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
            pending_seek: Some(Duration::from_micros(0)),
        })
    }

    fn ensure_media_set(&mut self) {
        if self.player.get_media().is_none() {
            self.player.set_media(&self.media);
        }

        match self.player.state() {
            State::Stopped | State::Ended | State::Error => self.player.set_media(&self.media),
            _ => (),
        }
    }

    pub fn play(&mut self) -> Result<(), Error> {
        self.ensure_media_set();

        if !self.playing()? {
            self.player.play().map_err(|_| {
                format_err!(
                    "Could not play media {:?}",
                    self.media.mrl().unwrap_or("<Could not obtain mrl>".into())
                )
            })?;
        }

        if !self.player.will_play() {
            bail!(
                "Player cannot currently play media {:?}",
                self.media.mrl().unwrap_or("<Could not obtain mrl>".into())
            );
        }

        self.last_pause_request = Some((Instant::now(), false));

        if let Some(to) = self.pending_seek.take() {
            self.seek(to);
        }

        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), Error> {
        if !self.playing()? {
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

    pub fn playing(&self) -> Result<bool, Error> {
        match self.last_pause_request {
            Some((at, paused)) if at.elapsed() < PAUSE_DIRTY_TIMEOUT => Ok(!paused),
            _ => match self.player.state() {
                State::Playing => Ok(true),
                State::Paused
                | State::Stopped
                | State::Ended
                | State::NothingSpecial
                | State::Error => Ok(false),
                State::Opening | State::Buffering => {
                    bail!("Cannot detect whether VLC is playing right now.")
                }
            },
        }
    }

    pub fn played(&self) -> Duration {
        match self.pending_seek {
            Some(seeking_to) => dbg!(seeking_to),
            None => match self.player.state() {
                State::Stopped | State::Ended | State::Error => self.duration(),
                _ => self
                    .player
                    .get_time()
                    // when player has time, convert it to duration
                    .map(|time| {
                        Duration::from_millis(time.try_into().expect("Player time out of bounds"))
                    })
                    // Note sure when this happens, logging it and assuming at the start
                    .unwrap_or_else(|| {
                        warn!(
                            "Could not get play time, assuming at the end. State: {:?}.",
                            self.player.state()
                        );
                        self.duration
                    }),
            },
        }
    }

    /// Full duration of the played media.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn seek(&mut self, from_start: Duration) {
        let from_start = min(self.duration(), from_start); // Skip to end if out of bounds

        self.ensure_media_set();
        if self.player.is_seekable()
            && self.player.state() != State::Stopped
            && self.player.state() != State::Ended
        {
            self.player.set_time(
                from_start
                    .as_millis()
                    .try_into()
                    .expect("Duration was out of bounds"),
            );
            self.pending_seek = None;
        } else {
            // Not seekable yet, maybe never played, do it when
            // playing the next time.
            self.pending_seek = Some(from_start);
        }
    }

    pub fn rewind(&mut self) {
        self.seek(Duration::from_millis(0));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    #[test]
    fn playing_lifecycle() {
        // given
        let mut player =
            Player::new("test/A Good Bass for Gambling.mp3").expect("Could not make player");
        let play_start_time = Instant::now();
        let end_offset = Duration::from_millis(100);
        let seek_pos = player.duration() - end_offset;
        let grace_time_end = Duration::from_millis(300);

        // when
        let played_before_play = player.played();
        player.play().expect("Could not play");
        assert!(player.playing().unwrap());

        while player.playing().unwrap() && play_start_time.elapsed() < Duration::from_secs(1) {
            sleep(Duration::from_secs(1))
        }
        assert!(play_start_time.elapsed() > Duration::from_secs(1));

        player.pause().unwrap();
        assert!(!player.playing().unwrap());
        sleep(PAUSE_DIRTY_TIMEOUT);
        assert!(!player.playing().unwrap());

        player.play().unwrap();
        assert!(player.playing().unwrap());
        sleep(PAUSE_DIRTY_TIMEOUT);
        assert!(player.playing().unwrap());

        player.seek(seek_pos);
        assert_eq!(player.played(), seek_pos);
        assert!(player.playing().unwrap());

        sleep(end_offset + grace_time_end);
        let playing_after_end = player.playing().unwrap();

        // then
        assert_eq!(played_before_play, Duration::from_millis(0));
        assert!(
            !playing_after_end,
            "Player should be paused after reaching the end of the media"
        );
    }

    #[test]
    fn can_rewind_after_finished() {
        // given
        let mut player = Player::new(crate::testutil::TEST_MUSIC).expect("Could not make player");

        let seek_from_end = Duration::from_millis(100);
        let load_grace_time = Duration::from_millis(2000);
        let seek_pos = player.duration() - seek_from_end;
        let wait_after_rewind_time = Duration::from_millis(500);

        // when
        player.seek(seek_pos);
        let played_before_play = player.played();
        player.play().expect("Could not play");
        let played_after_play = player.played();
        sleep(load_grace_time + seek_from_end);
        let played_after_end = player.played();
        player.rewind();
        let played_after_rewind = player.played();
        player.play().expect("Could not play");
        let played_after_rewind_and_play = player.played();
        sleep(wait_after_rewind_time);
        let played_after_rewind_and_play_and_wait = player.played();

        // then
        assert_eq!(
            played_before_play, seek_pos,
            "Expected to be at seek pos when seeking"
        );
        assert_eq!(
            played_after_play, seek_pos,
            "Expected to be at seek pos when seeking"
        );
        assert_ne!(
            played_after_end, seek_pos,
            "Expected played time to progress after seeking near the end"
        );
        assert_eq!(
            played_after_end,
            player.duration(),
            "Expected to be at duration when playing to the end"
        );
        assert_eq!(
            played_after_rewind,
            Duration::from_millis(0),
            "Expected to be at start when rewinding"
        );
        assert_eq!(
            played_after_rewind_and_play,
            Duration::from_millis(0),
            "Expected to be at start when rewinding"
        );
        assert!(
            played_after_rewind_and_play_and_wait > Duration::from_millis(200) &&
                played_after_rewind_and_play_and_wait <= (wait_after_rewind_time + Duration::from_millis(200)) ,
            "Expected to be progress normally after rewinding and playing, expected approx. at 500 ms, \
            but was: {:?}", played_after_rewind_and_play_and_wait
        );
    }
}
