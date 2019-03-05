use err::BellError;
use failure::Error;
use crate::act::Act;

pub struct Bell;

impl Act for Bell {
    fn done(&self) -> Result<bool, Error> {
        Err(BellError::not_installed())
    }

    fn cancel(&mut self) -> Result<(), Error> {
        Err(BellError::not_installed())
    }
}


mod err {
    use failure::{Fail, Error};

    #[derive(Fail, Debug)]
    pub enum BellError {
        #[fail(display = "The system has no bell installed")]
        NotInstalled
    }

    impl BellError {
        pub fn not_installed() -> Error {
            From::from(BellError::NotInstalled)
        }
    }
}
