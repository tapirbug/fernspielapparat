use crate::sense::Input;
use i2c_linux;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::time::Duration;

type Result<T> = std::result::Result<T, Error>;
type I2c = i2c_linux::I2c<File>;

pub struct Phone {
    i2c: I2c,
    /// Error code 121 is apparently returned from SMBus if
    /// no partner sent ACK. Retry a few times if this happens.
    retries_121: u64,
}

enum Msg {
    StartRing,
    StopRing,
}

impl Msg {
    fn into_u8(&self) -> u8 {
        match self {
            // FIXME the real bytes
            Msg::StartRing => 1,
            Msg::StopRing => 0,
        }
    }
}

impl Phone {
    pub fn new() -> Result<Self> {
        let mut i2c = I2c::from_path("/dev/i2c-1")?;
        i2c.smbus_set_slave_address(4, false)?;
        // Reads should not block for longer than ten millis so
        // writers get a chance to write, e.g. for ringing.
        i2c.i2c_set_timeout(Duration::from_millis(50))?;

        Ok(Phone {
            i2c,
            retries_121: 25,
        })
    }

    /// Tries to poll for input and takes a maximum of
    /// fifty milliseconds.
    ///
    /// For a healthy connection, this should always
    /// return something, e.g. consecutive hangups.
    pub fn poll(&mut self) -> Result<Input> {
        try_121_safe(self.retries_121, || self.i2c.smbus_read_byte()).and_then(Self::decode_input)
    }

    pub fn ring(&mut self) -> Result<()> {
        try_121_safe(self.retries_121, || self.send(Msg::StartRing))
    }

    pub fn unring(&mut self) -> Result<()> {
        try_121_safe(self.retries_121, || self.send(Msg::StopRing))
    }

    fn send(&mut self, msg: Msg) -> Result<()> {
        try_121_safe(self.retries_121, || {
            self.i2c
                .smbus_write_byte_data(msg.into_u8(), msg.into_u8())?;
            self.i2c.smbus_read_byte()?;
            Ok(())
        })
    }

    fn decode_input(byte: u8) -> Result<Input> {
        match byte {
            // FIXME the real bytes for digits
            digit @ b'0'..=b'9' => Ok(
                // Always in range [0,9], unwrap is safe
                Input::digit(digit - b'0').unwrap(),
            ),
            11 => Ok(Input::hang_up()),
            12 => Ok(Input::pick_up()),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Phone sent bad byte {}", byte),
            )),
        }
    }
}

fn try_121_safe<F, R>(retries: u64, mut trial: F) -> Result<R>
where
    F: FnMut() -> Result<R>,
{
    // Ignore errors retries minus 1 times
    for _ in 1..retries {
        match trial() {
            // Succeeded, ok
            ok @ Ok(_) => return ok,
            Err(e) => {
                if e.raw_os_error() == Some(121) {
                    // 121, this may still succeed later
                    ()
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
