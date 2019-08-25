use super::{App, Run, TerminalStateBehavior};

use crate::books::Book;
use crate::phone::Phone;
use crate::result::Result;
use crate::serve::Server;

use log::error;

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Mutex};

pub struct Builder {
    /// If `None`, starts with an idle run, otherwise
    /// starts with the phonebook.
    startup_book: Option<Book>,
    server: Option<Server>,
    phone: Option<Arc<Mutex<Phone>>>,
    terminal_state_behavior: TerminalStateBehavior,
    termination_flag: Arc<AtomicBool>,
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            startup_book: None,
            server: None,
            phone: None,
            terminal_state_behavior: TerminalStateBehavior::Rewind,
            // if never set up, termination flag never changes to true
            termination_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Builder {
    pub fn new() -> Builder {
        Default::default()
    }

    pub fn startup_phonebook(&mut self, book: Book) -> &mut Self {
        self.startup_book = Some(book);
        self
    }

    /// Tries to connect to phone at the given I2C device file, using
    /// the specified slave address.
    pub fn phone(&mut self, on_i2c_device: &str, address: u16) -> Result<&mut Self> {
        let phone = Phone::connect(on_i2c_device, address)?;
        self.phone = Some(Arc::new(Mutex::new(phone)));
        Ok(self)
    }

    /// Sets  a custom termination flag.
    pub fn termination_flag(&mut self, flag: &Arc<AtomicBool>) -> &mut Self {
        self.termination_flag = Arc::clone(flag);
        self
    }

    pub fn terminate_on_ctrlc_and_sigterm(&mut self) -> &mut Self {
        let termination_requested = Arc::new(AtomicBool::new(false));

        let termination_requested_handler_reference = Arc::clone(&termination_requested);
        let result = ctrlc::set_handler(move || {
            termination_requested_handler_reference.store(true, SeqCst);
        });

        match result {
            Ok(()) => self.termination_flag(&termination_requested),
            Err(e) => {
                error!(
                    "Failed to set up signal handler for safe termination. \
                     The phone may keep ringing after termination. \
                     Error: {:?}",
                    e
                );
                self
            }
        }
    }

    pub fn serve(&mut self, on_hostname_and_port: &str) -> Result<&mut Self> {
        self.server = Server::spawn(on_hostname_and_port).map(Some)?;
        Ok(self)
    }

    pub fn rewind_on_terminal_state(&mut self) -> &mut Self {
        self.terminal_state_behavior = TerminalStateBehavior::Rewind;
        self
    }

    pub fn exit_on_terminal_state(&mut self) -> &mut Self {
        self.terminal_state_behavior = TerminalStateBehavior::Exit;
        self
    }

    /// Consumes the builder and tries to create an app from it.
    ///
    /// This may fail, e.g. when the book references a sound file
    /// that is not present on the file system.
    pub fn build(self) -> Result<App> {
        let Builder {
            startup_book,
            server,
            phone,
            terminal_state_behavior,
            termination_flag,
        } = self;
        let server = server.map(Rc::new);

        let app = App {
            run: Run::new(startup_book, phone, server.as_ref().map(Rc::clone))?,
            server,
            terminal_state_behavior,
            termination_flag,
        };

        Ok(app)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::Ordering::SeqCst;

    #[test]
    fn build_with_default_settings() {
        // given
        let builder = App::builder();

        // when
        let app = builder.build().unwrap();

        // then
        assert!(app.server.is_none());
        assert_eq!(app.terminal_state_behavior, TerminalStateBehavior::Rewind);
        assert_eq!(app.termination_flag.load(SeqCst), false);
    }
}
