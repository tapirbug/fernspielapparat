use std::time::{Duration, Instant};
use tavla::{any_voice, Speech, Voice};

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
