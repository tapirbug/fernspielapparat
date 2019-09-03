use std::path::PathBuf;
use std::time::Duration;
use tavla::{any_voice, Speech, Voice};
use tempfile::tempdir;

pub use media::MediaInfo;

pub const TEST_MUSIC: &str = "test/A Good Bass for Gambling.mp3";
pub const _TEST_MUSIC_DURATION: Duration = Duration::from_micros(155_995250);

pub const WILHELM_SCREAM: &str = "test/482381__erokia__msfxp3-15-thunky-bass.wav";

/// Tolerance to account for variation in lag between VLC runs.
///
/// Most of the time it would work with half that tolerance, but
/// the loading and buffering time has some statistical outliers
/// every few runs, which this tolerance attempts to cover.
const TOLERANCE: Duration = Duration::from_millis(150);

pub fn assert_duration(topic: &str, expected: Duration, actual: Duration) {
    assert_duration_tolerance(topic, expected, actual, TOLERANCE)
}

fn assert_duration_tolerance(
    topic: &str,
    expected: Duration,
    actual: Duration,
    tolerance: Duration,
) {
    if actual > expected {
        let too_much = actual - expected;
        assert!(
            too_much < tolerance,
            "Expected {topic} of {expected:?}, instead got {actual:?}, which is too long by {excess:?}",
            topic = topic,
            expected = expected,
            actual = actual,
            excess = too_much
        )
    } else {
        let too_little = expected - actual;
        assert!(
            too_little < tolerance,
            "Expected {topic} of {expected:?}, actual: {actual:?}, is not long enough by {too_little:?}",
            topic = topic,
            expected = expected,
            actual = actual,
            too_little = too_little
        )
    }
}

/// Check how long it takes to speak the given string by actually
/// doing it and measuring.
pub fn actual_speech_time(for_str: &str) -> Duration {
    let tempdir = tempdir().expect("could not create temporary directory");
    let wav_file_path = {
        let mut wav_file_path: PathBuf = tempdir.path().into();
        wav_file_path.push("test_speech.wav");
        wav_file_path
    };

    let voice = any_voice().expect("Could not load voice to calculate expected timeout time");

    voice
        .speak_to_file(for_str, &wav_file_path)
        .expect("Failed to speak string into temporary file to calculate expected timeout time")
        .await_done()
        .expect("Failed to wait for speech end");

    MediaInfo::obtain(&wav_file_path)
        .expect("failed to play speech")
        .playing_duration()
}

mod media {
    use crate::result::Result;

    use crossbeam_channel::bounded;
    use failure::format_err;
    use vlc::{Instance, Media, MediaPlayer, MediaPlayerAudioEx, State};

    use std::cmp::max;
    use std::convert::TryInto;
    use std::iter::repeat_with;
    use std::path::Path;
    use std::time::{Duration, Instant};

    /// Measurements for buffering times, etc. so that tests can make informed
    /// decisions about tolerance.
    #[derive(Debug)]
    pub struct MediaInfo {
        load_time: Duration,
        buffering_lag: Duration,
        /// At which playback position does the player stop playing
        de_facto_duration: Duration,
        duration: Duration,
    }

    impl MediaInfo {
        pub fn obtain(for_file_at_path: impl AsRef<Path>) -> Result<MediaInfo> {
            /// Measure this many times and assume the worst lag of all
            const ITERATIONS: usize = 2;

            repeat_with(|| Self::measure(for_file_at_path.as_ref()))
                .take(ITERATIONS)
                .try_fold(Self::zero(), |acc, next| {
                    next.map(|next| Self::max_per_measurement(acc, next))
                })
        }

        /// Returns a combination of both parameters, taking the measurement
        /// with the worst lag from each.
        ///
        /// Duration is always taken from the second measurement.
        fn max_per_measurement(a: Self, b: Self) -> Self {
            MediaInfo {
                load_time: max(a.load_time, b.load_time),
                buffering_lag: max(a.buffering_lag, b.buffering_lag),
                de_facto_duration: max(a.de_facto_duration, b.de_facto_duration),
                duration: b.duration, // duration does not jitter, always the same, take the last
            }
        }

        fn zero() -> Self {
            MediaInfo {
                load_time: Duration::from_millis(0),
                buffering_lag: Duration::from_millis(0),
                de_facto_duration: Duration::from_millis(0),
                duration: Duration::from_millis(0),
            }
        }

        fn measure(for_file_at_path: &Path) -> Result<MediaInfo> {
            let load_start = Instant::now();

            let instance = &Instance::new().ok_or_else(|| {
                format_err!(
                    "Could not create VLC instance for playing {:?}",
                    for_file_at_path
                )
            })?;

            let media = Media::new_path(instance, for_file_at_path)
                .ok_or_else(|| format_err!("Could not load media {:?}", for_file_at_path))?;

            let player = MediaPlayer::new(instance)
                .ok_or_else(|| format_err!("Could not load media {:?}", for_file_at_path))?;

            let (duration, _) = Self::parse_media_duration(&media)?;

            player.set_media(&media);
            player.set_mute(true);

            let loading_lag = load_start.elapsed();
            let play_position_offset_after_loading = Self::measure_initial_lag(&player);
            let de_facto_duration = Self::measure_de_facto_media_duration(&player, duration);

            Ok(MediaInfo {
                load_time: loading_lag,
                buffering_lag: play_position_offset_after_loading,
                de_facto_duration,
                duration,
            })
        }

        fn parse_media_duration(media: &Media) -> Result<(Duration, Duration)> {
            let get_duration_start = Instant::now();

            let (duration_tx, duration_rx) = bounded::<Duration>(1);
            let evt_mgr = media.event_manager();
            evt_mgr
                .attach(vlc::EventType::MediaDurationChanged, move |e, _| {
                    if let vlc::Event::MediaDurationChanged(duration) = e {
                        duration_tx
                            .send(Duration::from_millis(duration.try_into().unwrap_or(0)))
                            .ok();
                    }
                })
                .map_err(|_| format_err!("Could not obtain media duration"))?;

            media.parse();

            Ok((
                duration_rx.recv_timeout(Duration::from_millis(500))?,
                get_duration_start.elapsed(),
            ))
        }

        /// Measure how much the actual playing position lags behind the
        /// actual time to expect if loading was instantenous.
        fn measure_initial_lag(player: &MediaPlayer) -> Duration {
            const MIN_PLAY_TIME: Duration = Duration::from_millis(500);

            let play_start = Instant::now();
            player.play().unwrap();

            // play for a known amount of real time
            std::thread::sleep(MIN_PLAY_TIME);

            let elapsed_on_clock = play_start.elapsed();
            let elapsed_in_player = player
                .get_time()
                .map(|t| Duration::from_millis(t.try_into().unwrap()))
                .unwrap();

            assert!(
                elapsed_on_clock > elapsed_in_player,
                "expected player to lag behind clock, not the other way around"
            );

            elapsed_on_clock - elapsed_in_player
        }

        /// In reality, media files either take longer than in the metadata to play
        /// fully, or they stop before the annotated duration.
        ///
        /// This measures the player position at which the player stops playing.
        fn measure_de_facto_media_duration(
            player: &MediaPlayer,
            media_duration: Duration,
        ) -> Duration {
            let media_duration_until_end = std::cmp::min(media_duration, Duration::from_secs(1));
            let seek_pos = media_duration - media_duration_until_end;

            assert!(player.is_seekable());
            player.set_time(seek_pos.as_millis().try_into().unwrap());

            let seek_start_time = Instant::now();
            let seek_pos = loop {
                let played = player.get_time().unwrap_or(0);
                let played = Duration::from_millis(played.try_into().unwrap());
                if played >= seek_pos {
                    break played;
                }
                if seek_start_time.elapsed() > Duration::from_millis(500) {
                    panic!("failed to seek");
                }
                std::thread::yield_now();
            };

            let playing_start_time = Instant::now();
            while player.state() == State::Playing {
                std::thread::yield_now();
            }
            let clock_time_until_done = playing_start_time.elapsed();

            seek_pos + clock_time_until_done
        }

        /// The detected media duration from VLC.
        pub fn media_duration(&self) -> Duration {
            self.duration
        }

        /// Expected time it takes to play the full file after it has
        /// been loaded, that is, the media duration has already been
        /// read.
        ///
        /// This accounts for buffering lag, that is, the difference
        /// between player position and the distance in time since the
        /// play button has been activated.
        pub fn playing_duration(&self) -> Duration {
            self.de_facto_duration
        }

        /// Actual duration, accounting for loading of the media,
        /// that is the construction of the player and reading the
        /// media duration, and also buffering lag, that is, the difference
        /// between player position and the distance in time since the
        /// play button has been activated.
        pub fn actual_duration(&self) -> Duration {
            self.load_time + self.de_facto_duration
        }

        /// How long approximately before player is ready to play.
        pub fn buffering_lag(&self) -> Duration {
            self.buffering_lag
        }
    }
}
