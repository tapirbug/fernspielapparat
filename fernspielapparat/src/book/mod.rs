use std::path::PathBuf;
use std::collections::HashMap;
use serde::Deserialize;

#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize)]
struct StateId(String);

#[derive(Deserialize)]
struct Book {
    states: HashMap<StateId, State>,
    #[serde(default)]
    transitions: HashMap<StateId, Transition>
}

#[derive(Deserialize)]
struct State {
    #[serde(default)]
    speech: String,
    speech_file: Option<PathBuf>,
    #[serde(default)]
    lights: Lighting
}

#[derive(Deserialize, Default)]
struct Lighting {
    #[serde(default)]
    power: i8,
    #[serde(default)]
    excitement: i8,
    #[serde(default)]
    mood: i8
}

#[derive(Deserialize)]
enum Transition {
    /// When input in some format was received.
    #[serde(rename = "dial")]
    Dial(HashMap<String, StateId>),
    /// When all actuators are done.
    #[serde(rename = "end")]
    Done(StateId)
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn deserialize_example_book() {
        let _book : Book = from_str(include_str!("../../test/testbook_full.yaml"))
            .expect("Expected the example book to work");
    }

    #[test]
    fn deserialize_without_initial_and_transitions() {
        let _book : Book = from_str(include_str!("../../test/testbook_only_states.yaml"))
            .expect("Could not deserialize");
    }

    #[should_panic]
    #[test]
    fn deserialize_empty_should_fail() {
        let _book : Book = from_str("")
            .expect("Could not deserialize");
    }
}