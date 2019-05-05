use crate::acts::Act;
use derivative::Derivative;
use failure::Error;
use log::warn;
use play::play;
use std::path::{Path, PathBuf};
use std::process::Child;

/// Plays a sound file in the background.
#[derive(Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct Sound {
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    child: Option<Child>,
    source: PathBuf,
}

impl Sound {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            child: play(path.as_ref())
                .map_err(|e| warn!("Could not play file {:?}, error: {}", path.as_ref(), e))
                .ok(),
            source: path.as_ref().into(),
        }
    }
}

impl Sound {
    pub fn source(&self) -> &Path {
        &self.source
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
    use failure::{format_err, Error};
    use std::path::Path;
    use std::process::{Child, Command};

    pub fn play<P: AsRef<Path>>(path: P) -> Result<Child, Error> {
        // paplay, cvlc, aplay, in that order
        ["paplay", "cvlc", "aplay"]
            .iter()
            .map(|app| Command::new(app).arg(path.as_ref()).spawn())
            .filter_map(Result::ok)
            .next()
            .ok_or_else(|| format_err!("Could not play audio file {:?}", path.as_ref()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
