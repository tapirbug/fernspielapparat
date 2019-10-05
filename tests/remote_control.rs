use std::thread::spawn;
use websocket::client::builder::ClientBuilder;
use websocket::OwnedMessage;

const SET_PHONEBOOK: &str = "{
    \"invoke\": \"run\",
    \"with\": {
        \"initial\":\"initial\",
        \"states\":{
            \"initial\":{},
            \"terminal\":{\"terminal\":true}
        },
        \"transitions\":{
            \"initial\":{\"end\": \"terminal\"}
        }
    }
}";

const DIAL_ONE: &str = "{
    \"invoke\": \"dial\",
    \"with\": \"1\"
}";

const START_ON_PASSIVE_EVT: &str = "---
type: start
initial:
  id: passive";
const START_ON_INITIAL_EVT: &str = "---
type: start
initial:
  id: initial";
const INITIAL_TO_TERMINAL_EVT: &str = "---
type: transition
reason:
  timeout: 0.0
from:
  id: initial
to:
  id: terminal";
const FINISH_ON_TERMINAL_EVT: &str = "---
type: finish
terminal:
  id: terminal";

const PHONEBOOK_WITH_DIAL_TRANSITION: &str = "---
initial: one
states:
  one:
    terminal: false
  two:
    terminal: true
transitions:
  one:
    dial:
      1: two";

const START_ON_ONE_EVT: &str = "---
type: start
initial:
  id: one";
const TRANSITION_TO_TWO_EVT: &str = "---
type: transition
reason:
  dial: type 1
from:
  id: one
to:
  id: two";
const FINISH_ON_TWO_EVT: &str = "---
type: finish
terminal:
  id: two";

#[test]
fn deploy_and_then_observe_transition() {
    fernspielapparat::log::init_logging(Some(3));

    // given
    let port = random_port();

    // when
    // start without startup phonebook
    spawn(move || {
        let mut app = fernspielapparat::App::builder();
        app.serve(&format!("127.0.0.1:{port}", port = port))
            .unwrap();
        app.exit_on_terminal_state();
        let mut app = app.build().unwrap();
        app.run().unwrap();
    });

    let mut client = ClientBuilder::new(&format!("ws:/127.0.0.1:{port}", port = port))
        .unwrap()
        .add_protocol("fernspielctl")
        .connect_insecure()
        .unwrap();

    client
        .send_message(&OwnedMessage::Text(SET_PHONEBOOK.to_string()))
        .unwrap();

    let mut incoming = client.incoming_messages();
    let event_start_passive = incoming
        .next()
        .expect("expected message for the transition to start next")
        .expect("expected ok message");
    let event_start_initial = incoming
        .next()
        .expect("expected message to start again at \"initial\", which was set via invocation")
        .expect("expected ok message");
    let event_transition_from_initial_to_terminal = incoming
        .next()
        .expect("expected message for transition to \"terminal\" from \"initial\"")
        .expect("expected ok message");
    let event_finish_terminal = incoming
        .next()
        .expect("expected message that the machine finished at \"terminal\"")
        .expect("expected ok message");

    client.send_message(&OwnedMessage::Close(None)).unwrap();
    client.shutdown().unwrap();

    // then
    assert_eq!(
        event_start_passive,
        OwnedMessage::Text(START_ON_PASSIVE_EVT.to_string())
    );
    assert_eq!(
        event_start_initial,
        OwnedMessage::Text(START_ON_INITIAL_EVT.to_string())
    );
    assert_eq!(
        event_transition_from_initial_to_terminal,
        OwnedMessage::Text(INITIAL_TO_TERMINAL_EVT.to_string())
    );
    assert_eq!(
        event_finish_terminal,
        OwnedMessage::Text(FINISH_ON_TERMINAL_EVT.to_string())
    );
}

#[test]
fn observe_transition_from_dial() {
    fernspielapparat::log::init_logging(Some(3));

    // given
    let port = random_port();

    // when
    let mut app = fernspielapparat::App::builder();
    app.startup_phonebook(
        fernspielapparat::books::from_str(PHONEBOOK_WITH_DIAL_TRANSITION).unwrap(),
    );
    app.serve(&format!("127.0.0.1:{port}", port = port))
        .unwrap();
    app.exit_on_terminal_state();
    spawn(move || {
        let mut app = app.build().unwrap();
        app.run().unwrap();
    });
    //std::thread::sleep(std::time::Duration::from_secs(5));
    let client = ClientBuilder::new(&format!("ws:/127.0.0.1:{port}", port = port))
        .unwrap()
        .add_protocol("fernspielctl")
        .connect_insecure()
        .expect("failed to make ws connection");
    let (mut rx, mut tx) = client.split().unwrap();

    let mut incoming = rx.incoming_messages();
    let event_start_passive = incoming
        .next()
        .expect("expected message of starting at the initial state")
        .expect("expected ok message");

    tx.send_message(&OwnedMessage::Text(DIAL_ONE.to_string()))
        .unwrap();

    let event_transition_two = incoming
        .next()
        .expect("expected message of a transition from \"one\" to \"two\" after dial")
        .expect("expected ok message");
    let event_finish_terminal = incoming
        .next()
        .expect("expected message that the machine finished at \"terminal\"")
        .expect("expected ok message");

    tx.send_message(&OwnedMessage::Close(None)).unwrap();
    tx.shutdown_all().unwrap();

    // then
    assert_eq!(
        event_start_passive,
        OwnedMessage::Text(START_ON_ONE_EVT.to_string())
    );
    assert_eq!(
        event_transition_two,
        OwnedMessage::Text(TRANSITION_TO_TWO_EVT.to_string())
    );
    assert_eq!(
        event_finish_terminal,
        OwnedMessage::Text(FINISH_ON_TWO_EVT.to_string())
    );
}

fn random_port() -> u32 {
    let rand: u32 = rand::random();
    10_000 + rand % 50_000
}
