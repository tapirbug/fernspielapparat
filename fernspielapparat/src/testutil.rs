use crate::result::Result;

use crossbeam_channel::bounded;
use failure::{bail, format_err};
use tavla::{any_voice, Speech, Voice};
use vlc::{Instance, Media, MediaPlayer};

use std::convert::TryInto;
use std::time::{Duration, Instant};

pub const TEST_MUSIC: &str = "test/A Good Bass for Gambling.mp3";
pub const _TEST_MUSIC_DURATION: Duration = Duration::from_micros(155_995250);

pub const WILHELM_SCREAM: &str = "test/Wilhelm_Scream.ogx";
pub const WILHELM_SCREAM_DURATION: Duration = Duration::from_micros(1_063673);

const TOLERANCE: Duration = Duration::from_millis(70);

pub fn assert_duration(topic: &str, expected: Duration, actual: Duration) {
    assert_duration_tolerance(topic, expected, actual, TOLERANCE)
}

pub fn assert_duration_tolerance(
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
    let voice = any_voice().expect("Could not load voice to calculate expected timeout time");

    let speech_start = Instant::now();

    voice
        .speak(for_str)
        .expect("Failed to speak string to calculate expected timeout time")
        .await_done()
        .expect("Failed to wait for speech end");

    speech_start.elapsed()
}

/// Measurements for buffering times, etc. so that tests can make informed
/// decisions about tolerance.
pub struct MediaInfo {
    buffering_lag: Duration,
    duration: Duration,
}

impl MediaInfo {
    pub fn obtain(for_file_at_path: &str) -> Result<MediaInfo> {
        const READ_DURATIONS_TIMEOUT: Duration = Duration::from_millis(500);

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

        let (duration_tx, duration_rx) = bounded::<Duration>(1);
        let (lag_tx, lag_rx) = bounded::<Duration>(1);
        let evt_mgr = media.event_manager();
        player
            .event_manager()
            .attach(vlc::EventType::MediaPlayerPlaying, move |e, _| {
                if let vlc::Event::MediaPlayerPlaying = e {
                    lag_tx.send(load_start.elapsed()).unwrap();
                }
            })
            .map_err(|_| {
                format_err!(
                    "Could not determine media loading lag: {:?}",
                    for_file_at_path
                )
            })?;
        evt_mgr
            .attach(vlc::EventType::MediaDurationChanged, move |e, _| {
                if let vlc::Event::MediaDurationChanged(duration) = e {
                    duration_tx
                        .send(Duration::from_millis(duration.try_into().unwrap_or(0)))
                        .unwrap();
                }
            })
            .map_err(|_| format_err!("Could not obtain media duration: {:?}", for_file_at_path))?;

        media.parse();

        let duration = duration_rx
            .recv_timeout(READ_DURATIONS_TIMEOUT)
            .map_err(|_| format_err!("Could not obtain media duration: {:?}", for_file_at_path))?;

        player.pause();
        player.set_media(&media);

        player.play().unwrap();
        while !player.is_playing() {
            if load_start.elapsed() > Duration::from_secs(3) {
                bail!("player did not become playable, aborting")
            }
            std::thread::yield_now();
        }

        let buffering_lag = lag_rx.recv_timeout(READ_DURATIONS_TIMEOUT).map_err(|_| {
            format_err!(
                "Could not determine media loading lag: {:?}",
                for_file_at_path
            )
        })?;

        player.pause();

        Ok(MediaInfo {
            buffering_lag,
            duration,
        })
    }

    /// The detected media duration from VLC
    pub fn media_duration(&self) -> Duration {
        self.duration
    }

    /// Actual duration, accounting for the approximate loading
    /// lag introduced when the file was scheduled to start but
    /// has not done enough buffering yet.
    pub fn actual_duration(&self) -> Duration {
        self.buffering_lag + self.duration
    }
}
