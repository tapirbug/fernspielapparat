use failure::{bail, Error};
use std::time::Duration;

/// Converts whole seconds specified as a float to a duration.
///
/// Maximum accuracy of the returned duration is microseconds,
/// sub-microseconds are truncated.
///
/// The maximum supported duration is 1.8446744 * 10^13 seconds,
/// where the microseconds stop fitting into `u64`. Overflow
/// is detected and an error result is returned.
///
/// Negative, `NaN` and infinite inputs also return an error.
pub fn to_duration(secs: f64) -> Result<Duration, Error> {
    if !secs.is_finite() {
        bail!(
            "Duration must be a finite, non-NaN number, instead got: {}",
            secs
        )
    } else if secs < 0.0 {
        bail!("Duration may not be negative: {}", secs)
    } else {
        const MAX_SECS: f64 = std::u64::MAX as f64;

        let whole_secs_floating = secs.trunc();
        if whole_secs_floating > MAX_SECS {
            bail!("Duration is too high, numeric overflow: {}", secs)
        }

        // nanos are always less than a million and cannot overflow
        let nanos = ((secs - whole_secs_floating) * 1_000_000.0) as u32;

        Ok(Duration::new(whole_secs_floating as u64, nanos))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn infinite_errs() {
        // given
        let duration = std::f64::NAN;

        // when
        let result = to_duration(duration);

        // then
        assert!(result.is_err(), "Expected NaN input to result in error",)
    }

    #[test]
    fn overflowing_whole_seconds_errs() {
        // given
        let duration = (std::u64::MAX as f64) * 2.0;

        // when
        let result = to_duration(duration);

        // then
        assert!(
            result.is_err(),
            "Expected overflowing whole seconds to result in error",
        )
    }

    #[test]
    fn negative_errs() {
        // given
        let duration = -0.1;

        // when
        let result = to_duration(duration);

        // then
        assert!(
            result.is_err(),
            "Expected overflowing whole seconds to result in error",
        )
    }
}
