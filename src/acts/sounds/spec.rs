pub use builder::*;

use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct SoundSpec {
    source: PathBuf,
    start_offset: Duration,
    end: EndBehavior,
    reenter: ReenterBehavior,
}

impl SoundSpec {
    pub fn builder() -> SoundSpecBuilderNeedingSource {
        SoundSpecBuilderNeedingSource
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

    pub fn start_offset(&self) -> Duration {
        self.start_offset
    }

    pub fn reenter_behavior(&self) -> ReenterBehavior {
        self.reenter
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum EndBehavior {
    Done,
    Loop,
}

impl Default for EndBehavior {
    fn default() -> Self {
        EndBehavior::Done
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReenterBehavior {
    /// Subtract duration from last playback position on re-enter.
    ///
    /// Re-enter occurs when a sound is activated and was not
    /// already activated.
    Backoff(Duration),
    /// Seek to this position on re-enter.
    ///
    /// Re-enter occurs when a sound is activated and was not
    /// already activated.
    Rewind,
}

impl Default for ReenterBehavior {
    fn default() -> Self {
        ReenterBehavior::Rewind
    }
}

mod builder {
    use super::*;

    use crate::result::Result;

    use failure::bail;

    pub struct SoundSpecBuilder {
        spec: SoundSpec,
    }

    /// Sound spec builder awaiting the specification of a
    /// source file before other properties can be set.
    ///
    /// Building is also not possible yet.
    pub struct SoundSpecBuilderNeedingSource;

    impl SoundSpecBuilderNeedingSource {
        pub fn source(&self, source: impl Into<PathBuf>) -> SoundSpecBuilder {
            SoundSpecBuilder {
                spec: SoundSpec {
                    source: source.into(),
                    start_offset: Duration::from_millis(0),
                    end: Default::default(),
                    reenter: Default::default(),
                },
            }
        }
    }

    impl SoundSpecBuilder {
        pub fn backoff(&mut self, backoff: impl Into<f64>) -> Result<&mut Self> {
            self.spec.reenter = ReenterBehavior::Backoff(f64_to_duration(backoff, "backoff")?);
            Ok(self)
        }

        pub fn start_offset(&mut self, backoff: impl Into<f64>) -> Result<&mut Self> {
            self.spec.start_offset = f64_to_duration(backoff, "start offset")?;
            Ok(self)
        }

        pub fn looping(&mut self, looping: bool) -> &mut Self {
            self.spec.end = if looping {
                EndBehavior::Loop
            } else {
                EndBehavior::Done
            };
            self
        }

        /// Builds the spec with the current config.
        ///
        /// Can be called multiple times without build influenceing
        /// each other.
        pub fn build(&mut self) -> SoundSpec {
            self.spec.clone()
        }
    }

    fn f64_to_duration(duration: impl Into<f64>, property_name: &str) -> Result<Duration> {
        let duration = duration.into();
        if duration < 0.0 {
            bail!(
                "Encountered negative {name}: {val}. \
                 Positive was expected.",
                name = property_name,
                val = duration
            )
        } else {
            // ms precision is ok here
            Ok(Duration::from_millis((duration * 1000.0) as u64))
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn negative_backoff() {
            let error = SoundSpecBuilderNeedingSource
                .source("/dev/null")
                .backoff(-0.0000001)
                .err();

            assert!(
                error.is_some(),
                "Negative backoff should be forbidden by error"
            );
        }

        #[test]
        fn negative_start_offset() {
            let error = SoundSpecBuilderNeedingSource
                .source("/dev/null")
                .start_offset(-0.0000001)
                .err();

            assert!(
                error.is_some(),
                "Negative offset should be forbidden by error"
            );
        }
    }
}
