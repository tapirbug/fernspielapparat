use failure::Error;
use crate::act::Act;

pub struct Bell;

impl Act for Bell {
    fn done(&self) -> Result<bool, Error> {
        unimplemented!()
    }

    fn cancel(&mut self) -> Result<(), Error> {
        unimplemented!()
    }
}
