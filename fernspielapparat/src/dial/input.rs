pub use err::Error;

/// Anything you can input with a phone dial,
/// including special characters like _#_,
/// picking up the speaker and hanging up.
#[derive(Debug, Clone, Copy)]
pub enum Input {
    /// A single digit number input in range [0,9].
    Digit(u8),
    PickUp,
    HangUp
}

impl Input {
    pub fn digit<N>(number: N) -> Result<Self, Error> where N : Into<i32> {
        let num = number.into();
        match num >= 0 && num < 10 {
            true  => Ok(Input::Digit(num as u8)),
            false => Err(Error::DigitOutOfBounds(num))
        }
    }

    pub fn pick_up() -> Self {
        Input::PickUp
    }

    pub fn hang_up() -> Self {
        Input::HangUp
    }

    /// If this is a number-like input, returns
    /// its numeric value.
    pub fn value(&self) -> Option<i32> {
        match self {
            Input::Digit(value) => Some(*value as i32),
            _ => None
        }
    }
}

mod err {
    use failure::Fail;

    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "digit {} was not in range [0,9]", _0)]
        DigitOutOfBounds(i32)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn value() {
        let five = Input::digit(5).unwrap();
        assert_eq!(5, five.value().unwrap());
    }

    #[should_panic]
    #[test]
    fn too_high_max_pluse_five() {
        let input : i32 = (std::u8::MAX as i32) + 5;
        Input::digit(input).unwrap();
    }

    #[should_panic]
    #[test]
    fn too_high_ten() {
        Input::digit(10).unwrap();
    }
}