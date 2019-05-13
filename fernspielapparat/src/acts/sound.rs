use crate::acts::Act;
use derivative::Derivative;
use failure::Error;
use log::warn;
use play::play;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

/// Plays a sound file in the background.
#[derive(Derivative)]
#[derivative(PartialEq, Eq, Hash, Debug)]
pub struct Sound {
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    child: Option<Child>,
    spec: SoundSpec,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct SoundSpec {
    source: PathBuf,
    end: EndBehavior,
    start_offset: Duration,
}

impl SoundSpec {
    fn new(source: PathBuf, end: EndBehavior, start_offset: Duration) -> Self {
        SoundSpec {
            source,
            end,
            start_offset,
        }
    }

    pub fn once<P: AsRef<Path>>(source: P, start_offset: Duration) -> Self {
        Self::new(source.as_ref().into(), EndBehavior::Done, start_offset)
    }

    pub fn repeat<P: AsRef<Path>>(source: P) -> Self {
        Self::new(
            source.as_ref().into(),
            EndBehavior::Loop,
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
    pub fn from_spec(spec: &SoundSpec) -> Self {
        Self {
            child: play(spec)
                .map_err(|e| warn!("Could not play file {:?}, error: {}", &spec.source, e))
                .ok(),
            spec: spec.clone(),
        }
    }
}

impl Act for Sound {
    fn update(&mut self) -> Result<(), Error> {
        if let Some(child) = self.child.as_mut() {
            let done = child.try_wait().map(|status| status.is_some())?;

            if done {
                self.child.take();
            }
        }
        Ok(())
    }

    fn done(&self) -> Result<bool, Error> {
        Ok(self.child.is_none())
    }

    fn cancel(&mut self) -> Result<(), Error> {
        self.child.take().as_mut().map(Child::kill);

        Ok(())
    }
}

mod play {
    use super::{EndBehavior::Done, EndBehavior::Loop, SoundSpec};
    use crate::version::detect_version;
    use failure::{format_err, Error};
    use log::debug;
    use std::process::{Child, Command};
    use std::time::Duration;

    pub fn play(spec: &SoundSpec) -> Result<Child, Error> {
        detect_version("cvlc")?;

        let mut cmd = Command::new("cvlc");
        cmd.arg("--no-one-instance");

        if let Done = spec.end {
            cmd.arg("--play-and-exit");
        }

        let is_loop = match spec.end {
            Loop => true,
            _ => false,
        };
        let has_offset = spec.start_offset != Duration::from_millis(0);
        if is_loop && has_offset {
            cmd.arg(&spec.source);
            cmd.arg(&format!(":start-time={}", fmt_seconds(&spec.start_offset)));
            cmd.arg(":no-loop");
            cmd.arg("--start-time=0");
        } else if is_loop {
            cmd.arg("--loop");
        } else if has_offset {
            cmd.arg(&format!("--start-time={}", fmt_seconds(&spec.start_offset)));
        }

        cmd.arg(&spec.source);

        debug!("Starting sound: {:?}", &cmd);

        cmd.spawn()
            .map_err(|e| format_err!("Could not play audio file {:?}, error: {}", &spec.source, e))
    }

    fn fmt_seconds(duration: &Duration) -> String {
        format!(
            "{secs}.{millis:03}",
            secs = duration.as_secs(),
            millis = duration.as_millis() % 1000
        )
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use std::thread::sleep;

        #[ignore]
        #[test]
        fn elevator_music() {
            let status = play(&SoundSpec {
                source: "test/A Good Bass for Gambling.mp3".into(),
                end: Done,
                start_offset: Duration::from_secs(2 * 60 + 30),
            })
            .expect("Could not play")
            .wait()
            .expect("Could not wait for end of music");

            assert!(status.success());
        }

        #[ignore]
        #[test]
        fn elevator_music_loop_then_stop() {
            let mut child = play(&SoundSpec {
                source: "test/A Good Bass for Gambling.mp3".into(),
                end: Loop,
                start_offset: Duration::from_secs(2 * 60 + 30),
            })
            .expect("Could not play");

            sleep(Duration::from_millis(10_000));

            child.kill().expect("Could not kill vlc");
        }

        #[test]
        fn fortytwo_point_041() {
            assert_eq!(
                fmt_seconds(&Duration::from_millis(42_041)),
                String::from("42.041")
            )
        }

    }
}

#[cfg(test)]
mod test {
    use super::*;
}
