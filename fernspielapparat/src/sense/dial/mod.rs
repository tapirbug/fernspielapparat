mod input;
mod stdin;

pub use input::Input;
pub use stdin::Stdin as StdinDial;

pub fn stdin_dial() -> StdinDial {
    StdinDial::new()
}
