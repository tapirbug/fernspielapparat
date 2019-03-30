use crate::sense::dial::Input;
use i2c_linux;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::time::Duration;

type Result<T> = std::result::Result<T, Error>;
type I2c = i2c_linux::I2c<File>;

pub struct Phone {
    i2c: I2c,
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
        // Reads should not block for longer than five millis so
        // writers get a chance to write, e.g. for ringing.
        i2c.i2c_set_timeout(Duration::from_millis(5))?;
        Ok(Phone { i2c })
    }

    /// Tries to poll for input and takes a maximum of
    /// five milliseconds.
    pub fn poll(&mut self) -> Result<Input> {
        self.i2c.smbus_read_byte().and_then(Self::decode_input)
    }

    pub fn ring(&mut self) -> Result<()> {
        self.send(Msg::StartRing)
    }

    pub fn unring(&mut self) -> Result<()> {
        self.send(Msg::StopRing)
    }

    fn send(&mut self, msg: Msg) -> Result<()> {
        self.i2c.smbus_write_byte(msg.into_u8())
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
