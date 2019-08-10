use crate::acts::Actuators;
use crate::books::Book;
use crate::phone::Phone;
use crate::senses::init_sensors;
use crate::states::Machine;

use failure::Error;

use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, Error>;

pub struct Run {
    /// Hold on to the book so the temp dir is preserved.
    _book: Book,
    machine: Machine,
    phone: Option<Arc<Mutex<Phone>>>,
}

impl Run {
    /// Makes the initial run, initializing the sensors and running
    /// the given optional book.
    ///
    /// If `None` is passed, a passive book is used until the next
    /// book switch.
    pub fn new(book: Option<Book>, phone: Option<Arc<Mutex<Phone>>>) -> Result<Self> {
        let book = book.unwrap_or_else(Book::passive);
        let sensors = init_sensors(&phone);
        let actuators = Actuators::new(&phone, book.sounds())?;
        let machine = Machine::new(sensors, actuators, book.states());

        let run = Run {
            _book: book,
            machine,
            phone,
        };

        Ok(run)
    }

    /// Keeps the current book open, but resets all actuators and
    /// starts over with the initial state.
    pub fn reset(&mut self) {
        self.machine.reset();
    }

    /// Continues evaluating the book.
    ///
    /// Depending on sensors, one transition may or may
    /// not be performed. Any additional transition only
    /// takes effect on the next tick, even if the conditions
    /// are met right away.
    pub fn tick(&mut self) -> bool {
        self.machine.update()
    }

    /// Consumes the given book and starts running it from the
    /// beginning, resetting any remaining actuator state.
    ///
    /// Any previously consumed book is dropped after the switch.
    ///
    /// If any error occurs, e.g. when the book references non-existing
    /// files, then the previous book remains in place.
    pub fn switch(&mut self, book: Book) -> Result<()> {
        // overwrite and reset the machine
        self.machine.load(
            Actuators::new(&self.phone, self._book.sounds())?,
            book.states(),
        );

        // and keep the book as it may contain temp dirs
        self._book = book;

        Ok(())
    }
}
