mod builder;
mod run;

use crate::result::Result;
use crate::senses::QueueInput;
use crate::serve::Request;
use crate::serve::Server;

use log::debug;
use run::Run;

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

pub use builder::Builder;

/// Controls the main loop, invoking the run for ticks
/// and controlling termination through the termination
/// flag and terminal states.
///
/// Manages interaction of the remote control server
/// with the application, replacing the run upon request.
pub struct App {
    /// Current run or a passive run if no real run ever
    /// happened.
    ///
    /// Can be modified by remote control messages.
    run: Run,
    server: Option<Rc<Server>>,
    /// Behavior when phonebook reaches a terminal state.
    terminal_state_behavior: TerminalStateBehavior,
    termination_flag: Arc<AtomicBool>,
    control: QueueInput,
}

#[derive(Debug, PartialEq)]
pub enum TerminalStateBehavior {
    /// When reaching a terminal state, exit the runtime
    /// with a successful exit status.
    Exit,
    /// When reaching a terminal state, start over at the
    /// initial state, resetting the phonebook.
    Rewind,
}

impl App {
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Keeps the application running, including phonebook evaluation
    /// and the remote control server, depending on configuration.
    /// Terminates when requested with termination flag or when reaching
    /// a fatal error, e.g. a malformed startup phonebook.
    ///
    /// Consumes the startup book.
    pub fn run(&mut self) -> Result<()> {
        while !self.should_terminate() {
            self.poll_remote_control()?;

            let running = self.run.tick();
            if !running {
                match self.terminal_state_behavior {
                    TerminalStateBehavior::Exit => {
                        debug!("reached terminal state, exiting");
                        break;
                    }
                    TerminalStateBehavior::Rewind => self.run.reset(),
                }
            }

            sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    fn poll_remote_control(&mut self) -> Result<()> {
        if let Some(server) = self.server.as_mut() {
            if let Some(request) = server.poll() {
                self.handle_request(request)?;
            }
        }

        Ok(())
    }

    /// Handles a websocket request, which may overwrite the current phonebook
    /// run.
    fn handle_request(&mut self, request: Request) -> Result<()> {
        match request {
            // reset request, start over with last phonebook
            Request::Reset => self.run.reset(),
            // stop current phonebook and launch the sent one
            Request::Run(new_book) => self.run.switch(new_book)?,
            Request::Dial(input) => {
                debug!("remote dial: {:?}", input);
                input.into_iter().for_each(|i| {
                    self.control.send(i).ok();
                })
            }
        };

        Ok(())
    }

    fn should_terminate(&self) -> bool {
        self.termination_flag.load(SeqCst)
    }
}
