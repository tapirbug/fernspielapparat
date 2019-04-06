use std::path::PathBuf;
use std::collections::HashMap;
use serde::Deserialize;
use std::fmt;

#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct StateId(String);

impl StateId {
    pub fn new<S: Into<String>>(from: S) -> Self {
        StateId(from.into())
    }
}

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize)]
pub struct Book {
    pub initial: StateId,
    pub terminal: Option<StateId>,
    pub states: HashMap<StateId, Option<State>>,
    #[serde(default)]
    pub transitions: HashMap<StateId, Transitions>
}

#[derive(Deserialize, Default)]
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

#[derive(Deserialize, Default)]
pub struct Transitions {
    /// When input in some format was received.
    #[serde(default)]
    pub dial: HashMap<u8, StateId>,
    pub pick_up: Option<StateId>,
    pub hang_up: Option<StateId>,
    /// When all actuators are done.
    pub end: Option<StateId>,
    pub timeout: Option<Timeout>
}

#[derive(Deserialize, Clone)]
pub struct Timeout {
    /// Time in seconds.
    pub after: f64,
    pub to: StateId
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
