
mod stdin;
mod input;

pub use input::Input;
pub use stdin::Dial as StdinDial;

pub fn stdin_dial() -> StdinDial {
    StdinDial::new()
}
