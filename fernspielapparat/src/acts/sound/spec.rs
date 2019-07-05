use std::path::{Path, PathBuf};
use std::time::Duration;

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

    pub fn reenter_behavior(&self) -> ReenterBehavior {
        if self.backoff.as_millis() > 0 {
            ReenterBehavior::Backoff(self.backoff)
        } else {
            ReenterBehavior::Seek(self.start_offset)
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum EndBehavior {
    Done,
    Loop,
}

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
    Seek(Duration)
}