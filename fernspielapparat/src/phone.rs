#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
pub use mock::*;

#[cfg(target_os = "linux")]
mod linux {
    use crate::senses::Input;
    use i2c_linux;
    use log::debug;
    use std::fs::File;
    use std::io::{Error, ErrorKind};
    use std::thread::sleep;
    use std::time::Duration;

    type Result<T> = std::result::Result<T, Error>;
    type I2c = i2c_linux::I2c<File>;

    // First wait 5ms, then 25, then 125, ... up  until 390_625ms
    const RETRIES: u32 = 8;
    const RETRY_BASE_MS: u64 = 5;

    pub struct Phone {
        i2c: I2c,
        /// Error code 121 is apparently returned from SMBus if
        /// no partner sent ACK. Retry a few times if this happens.
        retries: u32,
    }

    enum Msg {
        StartRing,
        StopRing,
        //ShortRing,
    }

    impl Msg {
        fn as_u8(&self) -> u8 {
            match self {
                //Msg::ShortRing => 2,
                Msg::StartRing => 1,
                Msg::StopRing => 0,
            }
        }
    }

    impl Phone {
        pub fn new() -> Result<Self> {
            let mut i2c = I2c::from_path("/dev/i2c-1")?;
            i2c.smbus_set_slave_address(4, false)?;

            Ok(Phone {
                i2c,
                retries: RETRIES,
            })
        }

        /// Tries to poll for input and takes a maximum of
        /// fifty milliseconds.
        ///
        /// For a healthy connection, this should always
        /// return something, e.g. consecutive hangups.
        pub fn poll(&mut self) -> Result<Input> {
            with_retries(self.retries, || self.i2c.smbus_read_byte_data(3)).and_then(Self::decode_input)
        }

        pub fn ring(&mut self) -> Result<()> {
            with_retries(self.retries, || {
                debug!("Ring start");
                self.send(Msg::StartRing)
            })
        }

        pub fn unring(&mut self) -> Result<()> {
            with_retries(self.retries, || {
                debug!("Ring end");
                self.send(Msg::StopRing)
            })
        }

        fn send(&mut self, msg: Msg) -> Result<()> {
            with_retries(self.retries, || {
                self.i2c.smbus_read_byte_data(msg.as_u8())?;
                Ok(())
            })
        }

        fn decode_input(byte: u8) -> Result<Input> {
            match byte {
                digit @ 0..=9 => Ok(
                    // Always in range [0,9], unwrap is safe
                    Input::digit(digit).unwrap(),
                ),
                // 10 => // TODO general error
                11 => Ok(Input::hang_up()),
                12 => Ok(Input::pick_up()),
                //13 => // TODO RECALL PRESS
                //14 => // TODO RECALL RELEASE
                255 => Err(Error::new(
                    ErrorKind::WouldBlock,
                    "Phone send buffer was empty",
                )),
                _ => Err(Error::new(
                    ErrorKind::WouldBlock,
                    format!("Phone sent bad byte {}", byte),
                )),
            }
        }
    }

    fn with_retries<F, R>(retries: u32, mut trial: F) -> Result<R>
    where
        F: FnMut() -> Result<R>,
    {
        // Ignore errors retries minus 1 times
        for attempt in 1_u32..retries {
            match trial() {
                // Succeeded, ok
                ok @ Ok(_) => return ok,
                Err(e) => {
                    if e.raw_os_error() == Some(121) {
                        // 121, this may still succeed later, retry with exponential backoff
                        sleep(Duration::from_millis(RETRY_BASE_MS.pow(attempt)))
                    } else {
                        // everything else is probably fatal
                        return Err(e);
                    }
                }
            }
        }

        // If the last is also 121, return it, or maybe we are lucky
        trial()
    }
}

/// Placeholder for a phone that can never be there because the target OS
/// is not linux, which is the only platform we support i2c for.
#[cfg(not(target_os = "linux"))]
mod mock {
    use std::io::{Error, ErrorKind};
    use crate::senses::Input;

    type Result<T> = std::result::Result<T, Error>;

    /// Can never be instantiated.
    pub enum Phone {}

    impl Phone {
        pub fn new() -> Result<Phone> {
            Err(
                Error::new(
                    ErrorKind::NotFound,
                    "I2C phone is not supported on this platform."
                )
            )
        }

        pub fn poll(&mut self) -> Result<Input> {
            unreachable!()
        }

        pub fn ring(&mut self) -> Result<()> {
            unreachable!()
        }

        pub fn unring(&mut self) -> Result<()> {
            unreachable!()
        }
    }
}
