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
const PICK_UP: &str = "{
    \"invoke\": \"dial\",
    \"with\": \"p\"
}";
const HANG_UP: &str = "{
    \"invoke\": \"dial\",
    \"with\": \"h\"
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

    let mut client = ClientBuilder::new(&format!("ws://127.0.0.1:{port}", port = port))
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
    let client = ClientBuilder::new(&format!("ws://127.0.0.1:{port}/", port = port))
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

#[test]
fn avoid_double_transition() {
    fernspielapparat::log::init_logging(Some(3));

    // given: a phonebook where a transition criterion is also a transition criterion on the target state
    let phonebook: &str = include_str!("../test/tv.yaml");
    let port = random_port();

    // when: picking up and then dialing one three times, then hanging up
    let mut app = fernspielapparat::App::builder();
    app.startup_phonebook(fernspielapparat::books::from_str(phonebook).unwrap());
    app.serve(&format!("127.0.0.1:{port}", port = port))
        .unwrap();
    app.exit_on_terminal_state();
    spawn(move || {
        let mut app = app.build().unwrap();
        app.run().unwrap();
    });
    let client = ClientBuilder::new(&format!("ws://127.0.0.1:{port}/", port = port))
        .unwrap()
        .add_protocol("fernspielctl")
        .connect_insecure()
        .expect("failed to make ws connection");
    let (mut rx, mut tx) = client.split().unwrap();

    let mut incoming = rx.incoming_messages();
    let _event_start_ring = incoming
        .next()
        .expect("expected message of starting at the initial state")
        .expect("expected ok message");
    tx.send_message(&OwnedMessage::Text(PICK_UP.to_string()))
        .unwrap();
    let event_transition_to_introduce = incoming
        .next()
        .expect("expected message of transition after pick up")
        .expect("expected ok message");
    tx.send_message(&OwnedMessage::Text(DIAL_ONE.to_string()))
        .unwrap();
    let event_transition_to_talk = incoming
        .next()
        .expect("expected message of transition after dialing one the first time")
        .expect("expected ok message");
    tx.send_message(&OwnedMessage::Text(DIAL_ONE.to_string()))
        .unwrap();
    let event_transition_to_quiet = incoming
        .next()
        .expect("expected message of transition after dialing one the second time")
        .expect("expected ok message");
    tx.send_message(&OwnedMessage::Text(DIAL_ONE.to_string()))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    let event_transition_to_talk_second_time = incoming
        .next()
        .expect("expected message of transition after dialing one the third time")
        .expect("expected ok message");
    tx.send_message(&OwnedMessage::Text(HANG_UP.to_string()))
        .unwrap();
    let event_transition_to_pause = incoming
        .next()
        .expect("expected message of transition after dialing one the second time")
        .expect("expected ok message");

    // then: there should be only two transitions triggered by the dialing of ones
    assert_eq!(
        event_transition_to_introduce,
        dial_transition_evt_msg("pick up", "RING", "INTRODUCE"),
        "Expecting picking up to result in transition to INTRODUCE"
    );
    assert_eq!(
        event_transition_to_talk,
        dial_transition_evt_msg("type 1", "INTRODUCE", "TALK"),
        "Expecting first dial of one to result in transition to TALK"
    );
    assert_eq!(
        event_transition_to_quiet,
        dial_transition_evt_msg("type 1", "TALK", "QUIET"),
        "Expecting second dial of one to result in transition to QUIET"
    );
    assert_eq!(
        event_transition_to_talk_second_time,
        dial_transition_evt_msg("type 1", "QUIET", "TALK"),
        "Expecting third dial of one to result in transition back to QUIET"
    );
    assert_eq!(
        event_transition_to_pause,
        dial_transition_evt_msg("hang up", "TALK", "PAUSE"),
        "Expecting hanging up to result in transition to PAUSE"
    );
}

fn random_port() -> u32 {
    let rand: u32 = rand::random();
    10_000 + rand % 50_000
}

fn dial_transition_evt_msg(dial: &str, from: &str, to: &str) -> OwnedMessage {
    OwnedMessage::Text(format!(
        "---
type: transition
reason:
  dial: {dial}
from:
  id: {from}
to:
  id: {to}",
        dial = dial,
        from = from,
        to = to
    ))
}
