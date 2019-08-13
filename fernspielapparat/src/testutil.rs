use std::time::Duration;

pub const TEST_MUSIC: &str = "test/A Good Bass for Gambling.mp3";

const TOLERANCE: Duration = Duration::from_millis(70);

pub fn assert_duration(topic: &str, expected: Duration, actual: Duration) {
    if actual > expected {
        let too_much = actual - expected;
        assert!(
            too_much < TOLERANCE,
            "Expected {topic} of {expected:?}, instead got {actual:?}, which is too long by {excess:?}",
            topic = topic,
            expected = expected,
            actual = actual,
            excess = too_much
        )
    } else {
        let too_little = expected - actual;
        assert!(
            too_little < TOLERANCE,
            "Expected {topic} of {expected:?}, actual: {actual:?}, is not long enough by {too_little:?}",
            topic = topic,
            expected = expected,
            actual = actual,
            too_little = too_little
        )
    }
}
