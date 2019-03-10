mod input;
mod stdin;

pub use input::Input;
pub use stdin::Dial as StdinDial;

pub fn stdin_dial() -> StdinDial {
    StdinDial::new()
}
