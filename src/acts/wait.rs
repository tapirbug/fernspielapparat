use crate::acts::Act;
use failure::Error;
use std::time::{Duration, Instant};

/// Act that does nothing but wait.
pub struct Wait {
    start: Instant,
    duration: Duration,
    done: bool,
}

impl Wait {
    pub fn new(duration: Duration) -> Self {
        Wait {
            start: Instant::now(),
            duration,
            done: duration == Duration::from_millis(0),
        }
    }
}

impl Act for Wait {
    fn activate(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn update(&mut self) -> Result<(), Error> {
        if !self.done && self.start.elapsed().gt(&self.duration) {
            self.done = true;
        }
        Ok(())
    }

    fn done(&self) -> Result<bool, Error> {
        Ok(self.done)
    }

    fn cancel(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn done_after_duration() {
        let duration = Duration::from_millis(200);
        let mut wait = Wait::new(duration);

        wait.update().unwrap();
        assert!(!wait.done().unwrap());
        wait.update().unwrap();
        sleep(duration);
        wait.update().unwrap();
        assert!(wait.done().unwrap());
    }

    #[test]
    fn not_done_after_90_percent_duration() {
        let duration = Duration::from_millis(200);
        let mut wait = Wait::new(duration);

        wait.update().unwrap();
        assert!(!wait.done().unwrap());
        wait.update().unwrap();
        sleep(duration / 10 * 9);
        wait.update().unwrap();
        assert!(!wait.done().unwrap());
    }

    #[test]
    fn zero_timeout_is_immediately_done() {
        assert!(Wait::new(Duration::from_micros(0)).done().unwrap())
    }

    #[test]
    fn non_zero_timeout_is_not_done_before_first_update() {
        let mut wait = Wait::new(Duration::from_micros(1));

        assert!(!wait.done().unwrap());
        sleep(Duration::from_millis(1));
        assert!(!wait.done().unwrap());
        wait.update().unwrap();
        assert!(wait.done().unwrap())
    }
}
