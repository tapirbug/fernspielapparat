use super::PlayerContext;
use failure::{bail, format_err, Error};
use log::warn;
use std::cmp::min;
use std::convert::TryInto;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::time::Instant;
use vlc::{self, Media, MediaPlayer, State};

const READ_DURATION_TIMEOUT: Duration = Duration::from_secs(4);
const PAUSE_DIRTY_TIMEOUT: Duration = Duration::from_millis(50);

/// Responsible for playback of a single file.
pub struct Player {
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
    /// When player context is not managed by client code, keep it here
    /// and free it when player is destroyed.
    _ctx: Option<PlayerContext>,
}

impl Player {
    /// Makes a new player that manages its own context.
    /// Currently only used in tests.
    ///
    /// When multiple players exist at the same time, it
    /// is more efficient to store the context outside the
    /// player and pass it in when creating new players.
    #[cfg(test)]
    pub fn new(file: impl AsRef<Path>) -> Result<Self, Error> {
        let ctx = PlayerContext::new()?;
        Self::new_with_ctx(file, &ctx).map(|mut p| {
            p.preserve_ctx(ctx);
            p
        })
    }

    /// Creates a new player with a caller-managed player
    /// context.
    pub fn new_with_ctx(file: impl AsRef<Path>, ctx: &PlayerContext) -> Result<Self, Error> {
        let instance = ctx.vlc_instance();

        let media = Media::new_path(instance, file.as_ref())
            .ok_or_else(|| format_err!("Could not load media {:?}", file.as_ref()))?;

        let player = MediaPlayer::new(instance)
            .ok_or_else(|| format_err!("Could not load media {:?}", file.as_ref()))?;

        let (tx, rx) = channel::<Duration>();
        media
            .event_manager()
            .attach(vlc::EventType::MediaDurationChanged, move |e, _| {
                if let vlc::Event::MediaDurationChanged(duration) = e {
                    tx.send(Duration::from_millis(duration.try_into().unwrap_or(0)))
                        .ok();
                }
            })
            .map_err(|_| format_err!("Could not obtain media duration: {:?}", file.as_ref()))?;

        media.parse();

        let duration = rx
            .recv_timeout(READ_DURATION_TIMEOUT)
            .map_err(|_| format_err!("Could not obtain media duration: {:?}", file.as_ref()))?;

        Ok(Player {
            media,
            player,
            duration,
            last_pause_request: None,
            pending_seek: Some(Duration::from_micros(0)),
            _ctx: None,
        })
    }

    #[cfg(test)]
    fn preserve_ctx(&mut self, ctx: PlayerContext) {
        self._ctx = Some(ctx);
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
                    self.media
                        .mrl()
                        .unwrap_or_else(|| "<Could not obtain mrl>".into())
                )
            })?;
        }

        if !self.player.will_play() {
            bail!(
                "Player cannot currently play media {:?}",
                self.media
                    .mrl()
                    .unwrap_or_else(|| "<Could not obtain mrl>".into())
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
                self.media
                    .mrl()
                    .unwrap_or_else(|| "<Could not obtain mrl>".into())
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
            Some(seeking_to) => seeking_to,
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

    use crate::testutil::{assert_duration, MediaInfo, TEST_MUSIC, WILHELM_SCREAM};

    use std::thread::sleep;
    use std::time::{Duration, Instant};

    /// Checks that the player does not report to be playing before play
    /// was called. Playing time also should not increase.
    #[test]
    fn no_progress_before_play() {
        // given
        const WAIT_TIME: Duration = Duration::from_millis(300);

        // when
        let player = Player::new(TEST_MUSIC).expect("could not make player");
        let played_before_play = player.played();
        let playing_before_play = player
            .playing()
            .expect("could not check if playing before play");

        sleep(WAIT_TIME);

        let played_before_play_after_wait = player.played();
        let playing_before_play_after_wait = player
            .playing()
            .expect("could not check if playing before play");

        // then
        assert_eq!(
            played_before_play,
            Duration::from_millis(0),
            "Expected zero playing time before play"
        );
        assert_eq!(
            played_before_play_after_wait,
            Duration::from_millis(0),
            "Expected zero playing time before play, even when waiting (no autoplay)"
        );
        assert!(
            !playing_before_play,
            "Expect playing to be false before play"
        );
        assert!(
            !playing_before_play_after_wait,
            "Expect playing to be false before play, even when waiting (no autoplay)"
        );
    }

    /// Starts playing and checks if behaves normally for the first second.
    /// Then pauses the player. Waits a bit and checks if it is still paused.
    #[test]
    fn play_then_pause() {
        // given
        const PLAY_TIME: Duration = Duration::from_millis(500);
        const WAIT_TIME_AFTER_PAUSE: Duration = Duration::from_millis(600);
        let media_info = MediaInfo::obtain(TEST_MUSIC).unwrap();
        let expected_play_time_minus_loading_lag = PLAY_TIME - media_info.buffering_lag();

        // when
        let mut player = Player::new(TEST_MUSIC).expect("could not make player");
        player.play().expect("could not play");

        let played_after_play = player.played();
        let playing_after_play = player
            .playing()
            .expect("could not check if playing after play");

        sleep(PLAY_TIME);

        let played_after_play_and_wait = player.played();
        let playing_after_play_and_wait = player
            .playing()
            .expect("could not check if playing after play and waiting");

        player.pause().expect("could not pause");

        let played_after_pause = player.played();
        let playing_after_pause = player.playing().unwrap();

        sleep(WAIT_TIME_AFTER_PAUSE);

        let played_after_pause_and_wait = player.played();
        let playing_after_pause_and_wait = player
            .playing()
            .expect("could not check if playing before play");

        // then
        assert_eq!(
            played_after_play,
            Duration::from_millis(0),
            "Expected no progress immediately after playing"
        );
        assert_duration(
            "playback position",
            expected_play_time_minus_loading_lag,
            played_after_play_and_wait,
        );
        assert!(
            playing_after_play,
            "Expected player to immediately report to be playing after calling play"
        );
        assert!(
            playing_after_play_and_wait,
            "Expected player to still be playing after playing for half a second"
        );
        assert_duration(
            "playing time after pausing",
            played_after_play_and_wait,
            played_after_pause,
        );
        assert!(
            !playing_after_pause,
            "Expected player to not report being played after pausing"
        );
        assert!(
            !playing_after_pause_and_wait,
            "Expected player to not report being played after pausing and then waiting a bit"
        );
        assert_eq!(
            played_after_pause, played_after_pause_and_wait,
            "Expected playing time not to change when waiting after pause"
        );
    }

    /// Checks that playing a media file of a known duration does
    /// not take significantly longer to actually play.
    #[test]
    fn playing_duration_wilhelm_scream() {
        // given
        const PLAY_CHECK_INTERVAL: Duration = Duration::from_millis(10);
        let expected_duration = MediaInfo::obtain(WILHELM_SCREAM)
            .expect("could not inspect wilhelm scream")
            // buffering lag is acceptable and platform dependent, measure it first
            // so we have an estimate how much needs to be compensated
            .playing_duration();
        let max_play_loop_time = expected_duration + Duration::from_secs(2);

        // when
        let mut player = Player::new(WILHELM_SCREAM).expect("could not make player");
        let player_start_time = Instant::now();
        player.play().expect("could not play");
        while player_start_time.elapsed() < max_play_loop_time && player.playing().unwrap() {
            sleep(PLAY_CHECK_INTERVAL);
        }
        let playing_time = player_start_time.elapsed();

        // then
        assert_duration(
            "wilhelm scream playing time",
            expected_duration,
            playing_time,
        );
    }

    /// Starts playing and fast forwards to near the end.
    /// Checks if stops after reaching the end.
    #[test]
    fn fast_forward() {
        // given
        const END_OFFSET: Duration = Duration::from_millis(100);
        const MAX_PLAY_LOOP_TIME: Duration = Duration::from_secs(2);
        const PLAY_CHECK_INTERVAL: Duration = Duration::from_millis(10);
        const WAIT_TIME_AFTER_END: Duration = PAUSE_DIRTY_TIMEOUT;

        let media_duration = MediaInfo::obtain(TEST_MUSIC).unwrap().media_duration();
        let seek_pos = media_duration - END_OFFSET;

        // when
        let mut player = Player::new(TEST_MUSIC).expect("Could not make player");

        player.play().expect("Could not play");
        player.seek(seek_pos);

        let play_start_time = Instant::now();
        while player.playing().unwrap() && play_start_time.elapsed() < MAX_PLAY_LOOP_TIME {
            sleep(PLAY_CHECK_INTERVAL);
        }
        let playing_time = play_start_time.elapsed();
        let played_time = player.played();
        let playing_after_end = player.playing().unwrap();

        sleep(WAIT_TIME_AFTER_END);

        let played_time_after_wait = player.played();
        let playing_after_end_after_wait = player.playing().unwrap();

        // then
        assert!(
            !playing_after_end,
            "Player should be paused after reaching the end of the media"
        );
        assert_duration(
            "playing time after seeking near end",
            END_OFFSET,
            playing_time,
        );
        assert_eq!(
            played_time, media_duration,
            "Expected player to report having played exactly the media length"
        );
        assert_eq!(playing_after_end, playing_after_end_after_wait);
        assert_eq!(played_time, played_time_after_wait);
    }

    #[test]
    fn can_rewind_after_finished() {
        // given
        let seek_from_end = Duration::from_millis(100);
        let load_grace_time = Duration::from_millis(2000);
        let seek_pos = MediaInfo::obtain(TEST_MUSIC).unwrap().media_duration() - seek_from_end;
        let wait_after_rewind_time = Duration::from_millis(500);

        // when
        let mut player = Player::new(TEST_MUSIC).expect("Could not make player");
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
