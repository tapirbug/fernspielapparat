use crate::senses::{dial::Input, Error};

type Result<T> = std::result::Result<T, Error>;

pub trait Sense {
    /// Tries to get the next input from the sensor in a
    /// way that may block.
    ///
    /// When an error is returned, it is assumed non-recoverable.
    fn poll(&mut self) -> Result<Input>;
}
