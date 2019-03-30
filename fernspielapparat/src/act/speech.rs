use crate::act::Act;
use failure::Error;
use tavla::Speech;

/// Speech errors are `Fail`, we can convert them
/// to failure errors.
impl<T: Speech> Act for T {
    fn update(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn done(&self) -> Result<bool, Error> {
        <T as Speech>::is_done(self).map_err(From::from)
    }

    fn cancel(&mut self) -> Result<(), Error> {
        <T as Speech>::cancel(self).map_err(From::from)
    }
}
