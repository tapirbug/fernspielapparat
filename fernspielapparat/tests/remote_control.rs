use std::thread::spawn;
use std::time::Instant;
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

#[test]
fn deploy_and_then_observe_transition() {
    // given
    let port = random_port();

    // when
    //cute_log::init_with_max_level(log::LevelFilter::Debug);

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

fn random_port() -> u32 {
    let rand: u32 = rand::random();
    10_000 + rand % 50_000
}
