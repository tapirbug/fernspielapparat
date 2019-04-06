use std::path::PathBuf;
use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;

#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct StateId(String);

#[derive(Deserialize)]
pub struct Book {
    pub states: HashMap<StateId, Option<State>>,
    #[serde(default)]
    pub transitions: HashMap<StateId, Transitions>
}

#[derive(Deserialize)]
pub struct State {
    #[serde(default)]
    pub speech: String,
    pub speech_file: Option<PathBuf>,
    #[serde(default)]
    pub lights: Lighting,
    /// Ringing time in seconds
    #[serde(default)]
    pub ring: f64
}

#[derive(Deserialize, Default)]
pub struct Lighting {
    #[serde(default)]
    pub power: i8,
    #[serde(default)]
    pub excitement: i8,
    #[serde(default)]
    pub mood: i8
}

#[derive(Deserialize)]
pub struct Transitions {
    /// When input in some format was received.
    #[serde(default)]
    dial: HashMap<u8, StateId>,
    pick_up: Option<StateId>,
    hang_up: Option<StateId>,
    /// When all actuators are done.
    end: Option<StateId>,
    timeout: Option<Timeout>
}

#[derive(Deserialize)]
struct Timeout {
    /// Time in seconds.
    after: f64,
    to: StateId
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
    fn deserialize_default_book() {
        let _book : Book = from_str(include_str!("../../resources/default.yaml"))
            .expect("Expected the default book to work");
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
