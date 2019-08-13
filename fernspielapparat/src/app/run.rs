use crate::acts::Actuators;
use crate::books::Book;
use crate::evt::Responder;
use crate::phone::Phone;
use crate::senses::init_sensors;
use crate::serve::{EventPublisher, Server};
use crate::states::State;

use failure::Error;

use std::rc::Rc;
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, Error>;
type CompositeResponder = crate::evt::CompositeResponder<State>;
type Machine = crate::states::Machine<CompositeResponder>;

pub struct Run {
    /// Hold on to the book so the temp dir is preserved.
    book: Book,
    machine: Machine,
    phone: Option<Arc<Mutex<Phone>>>,
    server: Option<Rc<Server>>,
}

impl Run {
    /// Makes the initial run, initializing the sensors and running
    /// the given optional book.
    ///
    /// If `None` is passed, a passive book is used until the next
    /// book switch.
    pub fn new(
        book: Option<Book>,
        phone: Option<Arc<Mutex<Phone>>>,
        server: Option<Rc<Server>>,
    ) -> Result<Self> {
        let book = book.unwrap_or_else(Book::passive);
        let sensors = init_sensors(&phone);
        let responder = Self::make_responders_inner(&phone, &server, &book)?;
        let machine = Machine::new(sensors, responder, book.states());

        let run = Run {
            book,
            machine,
            phone,
            server: server.clone(),
        };

        Ok(run)
    }

    fn make_responders(&self) -> Result<CompositeResponder> {
        Self::make_responders_inner(&self.phone, &self.server, &self.book)
    }

    fn make_responders_inner(
        phone: &Option<Arc<Mutex<Phone>>>,
        server: &Option<Rc<Server>>,
        book: &Book,
    ) -> Result<CompositeResponder> {
        let mut responders: Vec<Box<dyn Responder<State>>> = Vec::with_capacity(2);

        let actuators = Actuators::new(phone, book.sounds())?;
        responders.push(Box::new(actuators));

        if let Some(server) = server.as_ref() {
            let publisher = EventPublisher::through(server);
            responders.push(Box::new(publisher));
        }

        Ok(CompositeResponder::from(responders))
    }

    /// Keeps the current book open, but resets all actuators and
    /// starts over with the initial state.
    pub fn reset(&mut self) {
        self.machine.reset();
    }

    /// Continues evaluating the book.
    ///
    /// Returns `false` when a terminal state is current, otherwise
    /// `true`.
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
        self.machine.load(self.make_responders()?, book.states());

        // and keep the book as it may contain temp dirs
        self.book = book;

        Ok(())
    }
}
