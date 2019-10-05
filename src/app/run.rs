use crate::acts::Actuators;
use crate::books::Book;
use crate::evt::Responder;
use crate::phone::Phone;
use crate::result::Result;
use crate::senses::QueueInput;
use crate::senses::{Sensors, SensorsBuilder};
use crate::serve::{EventPublisher, Server};
use crate::states::State;

use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    pub fn new_with_queue(
        book: Option<Book>,
        phone: Option<Arc<Mutex<Phone>>>,
        server: Option<Rc<Server>>,
    ) -> Result<(Self, QueueInput)> {
        let mut sensors = init_sensors(&phone);
        let (_, queue) = sensors.queue();
        Self::new_with_sensors(book, phone, server, sensors).map(|r| (r, queue))
    }

    fn new_with_sensors(
        book: Option<Book>,
        phone: Option<Arc<Mutex<Phone>>>,
        server: Option<Rc<Server>>,
        sensors: SensorsBuilder,
    ) -> Result<Self> {
        let book = book.unwrap_or_else(Book::passive);
        let sensors = sensors.build();
        let responder = make_responder(&phone, &server, &book)?;
        let machine = Machine::new(sensors, responder, book.states());

        let run = Run {
            book,
            machine,
            phone,
            server: server.clone(),
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
        let responders = make_responder(&self.phone, &self.server, &book)?;
        self.machine.load(responders, book.states());

        // and keep the book as it may contain temp dirs
        self.book = book;

        Ok(())
    }
}

#[cfg(test)]
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
        let sensors = init_sensors(&phone);
        Self::new_with_sensors(book, phone, server, sensors)
    }
}

fn make_responder(
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

pub fn init_sensors(phone: &Option<Arc<Mutex<Phone>>>) -> SensorsBuilder {
    let mut sensors = Sensors::builder();
    sensors.stdin();

    if let Some(phone) = phone.as_ref() {
        sensors.i2c_dial(phone);
    }
    sensors
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::books::spec::Sound as SoundSpec;
    use crate::log::init_test_logging;
    use crate::senses::Input;
    use crate::testutil::{
        actual_speech_time, assert_duration, MediaInfo, TEST_MUSIC, WILHELM_SCREAM,
    };
    use std::thread::yield_now;
    use std::time::Instant;

    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    #[test]
    fn switch_to_new_speech() {
        // given
        init_test_logging();
        let old_text = "...";
        let mut book_with_one_sound = Book::builder();
        book_with_one_sound
            .sound(speech(old_text))
            .unwrap()
            .state(
                State::builder()
                    .id("Book 1 State with index 0")
                    .name("Book 1 State with index 0")
                    .sounds(vec![0])
                    .end(1)
                    .build(),
            )
            .state(
                State::builder()
                    .id("Book 1 State with index 1")
                    .name("Book 1 State with index 1")
                    .terminal(true)
                    .build(),
            );
        let book_with_one_sound = book_with_one_sound.build();
        let new_text = "hey there, just loaded";
        let new_text_duration = actual_speech_time(new_text);
        let mut book_with_two_sounds = Book::builder();
        book_with_two_sounds
            .sound(speech(new_text))
            .unwrap()
            .sound(speech(new_text))
            .unwrap()
            .state(
                State::builder()
                    .id("Book 2 State with index 0")
                    .name("Book 2 State with index 0")
                    .sounds(vec![0, 1])
                    .end(1)
                    .build(),
            )
            .state(
                State::builder()
                    .id("Book 2 State with index 1")
                    .name("Book 2 State with index 1")
                    .terminal(true)
                    .build(),
            );
        let book_with_two_sounds = book_with_two_sounds.build();

        // when
        let mut run = Run::new(Some(book_with_one_sound), None, None).unwrap();
        let initial_sounds = &run.book.sounds().to_vec();
        let initially_busy = run.tick();
        run.switch(book_with_two_sounds).unwrap();
        let busy_after_switch = run.tick();
        let new_sounds = &run.book.sounds().to_vec();
        let new_state_tick_start = Instant::now();
        while run.tick() {
            yield_now();
        } // should reach terminal state in one second
        let new_state_tick_duration = new_state_tick_start.elapsed();

        // then
        assert!(
            initially_busy,
            "run tick already returns false form tick but expected to be busy playing music"
        );
        assert!(
            busy_after_switch,
            "run tick already returns false after switch but expected to be busy playing music"
        );
        assert_ne!(initial_sounds, new_sounds);
        assert_eq!(initial_sounds.len(), 1);
        assert_eq!(new_sounds.len(), 2);
        assert_duration(
            "evaluation time",
            new_text_duration,
            new_state_tick_duration,
        );
    }

    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    #[test]
    fn switch_to_new_music_non_looping() {
        // given
        init_test_logging();
        let scream_info = MediaInfo::obtain(WILHELM_SCREAM).unwrap();

        let mut book_with_one_sound = Book::builder();
        book_with_one_sound
            .sound(music_non_looping(TEST_MUSIC))
            .unwrap()
            .sound(music_non_looping(TEST_MUSIC))
            .unwrap()
            .state(
                State::builder()
                    .id("Book 1 State with index 0")
                    .name("Book 1 State with index 0")
                    .sounds(vec![0])
                    .end(1)
                    .build(),
            )
            .state(
                State::builder()
                    .id("Book 1 State with index 1")
                    .name("Book 1 State with index 1")
                    .terminal(true)
                    .build(),
            );
        let book_with_one_sound = book_with_one_sound.build();
        let mut book_with_two_sounds = Book::builder();
        book_with_two_sounds
            .sound(music_non_looping(WILHELM_SCREAM))
            .unwrap()
            .state(
                State::builder()
                    .id("Book 2 State with index 0")
                    .name("Book 2 State with index 0")
                    .sounds(vec![0])
                    .end(1)
                    .build(),
            )
            .state(
                State::builder()
                    .id("Book 2 State with index 1")
                    .name("Book 2 State with index 1")
                    .terminal(true)
                    .build(),
            );
        let book_with_two_sounds = book_with_two_sounds.build();

        // when
        let mut run = Run::new(Some(book_with_one_sound), None, None).unwrap();
        let initial_sounds = &run.book.sounds().to_vec();
        let initially_busy = run.tick();
        run.switch(book_with_two_sounds).unwrap();
        let busy_after_switch = run.tick();
        let new_sounds = &run.book.sounds().to_vec();
        let new_state_tick_start = Instant::now();
        while run.tick() {
            yield_now();
        } // should reach terminal state in one second
        let new_state_tick_duration = new_state_tick_start.elapsed();

        // then
        assert!(
            initially_busy,
            "run tick already returns false form tick but expected to be busy playing music"
        );
        assert!(
            busy_after_switch,
            "run tick already returns false after switch but expected to be busy playing music"
        );
        assert_ne!(initial_sounds, new_sounds);
        assert_eq!(initial_sounds.len(), 2);
        assert_eq!(new_sounds.len(), 1);
        assert_duration(
            "evaluation time",
            scream_info.playing_duration(),
            new_state_tick_duration,
        );
    }

    #[test]
    fn switch_from_queue_dial() {
        // given
        let mut book = Book::builder();
        book.state(
            State::builder()
                .id("1")
                .name("1")
                .input(Input::pick_up(), 1)
                .build(),
        )
        .state(State::builder().id("2").name("2").terminal(true).build());
        let book = book.build();

        // when
        let (mut run, input) = Run::new_with_queue(Some(book), None, None).unwrap();
        let initially_running = run.tick();
        input.send(Input::pick_up()).ok();
        let running_after_pick_up = run.tick();

        // then
        assert!(
            initially_running,
            "Expected initial state to remain in place"
        );
        assert!(
            !running_after_pick_up,
            "Sent asnychroneous pick up, but did not see expected transition to terminal state"
        )
    }

    fn speech(speech: &str) -> SoundSpec {
        SoundSpec {
            speech: Some(speech.into()),
            file: String::new(),
            volume: 1.0,
            backoff: None,
            looping: false,
            start_offset: None,
        }
    }

    fn music_non_looping(music_file: &str) -> SoundSpec {
        SoundSpec {
            speech: None,
            file: music_file.to_string(),
            volume: 1.0,
            backoff: None,
            looping: false,
            start_offset: None,
        }
    }
}
