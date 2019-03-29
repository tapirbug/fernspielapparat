use i2c_linux;
use std::io::{Error, ErrorKind};
use std::fs::File;
use crate::sense::dial::Input;

type Result<T> = std::result::Result<T, Error>;
type I2c = i2c_linux::I2c<File>;

pub struct Phone {
    i2c: I2c
}

enum Msg {
    StartRing,
    StopRing
}

impl Msg {
    fn into_u8(&self) -> u8 {
        match self {
            // FIXME the real bytes
            Msg::StartRing => b'\x07',
            Msg::StopRing => b'\0'
        }
    }
}

impl Phone {
    pub fn new() -> Result<Self> {
        let i2c = I2c::from_path("/dev/i2c-1")?;
        Ok(Phone { i2c })
    }

    pub fn poll(&mut self) -> Result<Input> {
        self.i2c.smbus_read_byte()
            .and_then(Self::decode_input)
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
            // FIXME the real bytes
            digit@b'0'..=b'9' => Ok(
                // Always in range [0,9], unwrap is safe
                Input::digit(digit - b'0').unwrap()
            ),
            b'p' => Ok(Input::pick_up()),
            b'h' => Ok(Input::hang_up()),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Phone sent bad byte {}", byte)
            ))
        }
    }
}
