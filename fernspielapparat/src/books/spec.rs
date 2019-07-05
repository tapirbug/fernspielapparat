use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Id(String);

impl Id {
    pub fn new<S: Into<String>>(from: S) -> Self {
        Id(from.into())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Deserialize)]
pub struct Book {
    pub initial: Id,
    pub states: HashMap<Id, Option<State>>,
    #[serde(default)]
    pub transitions: HashMap<Id, Transitions>,
    #[serde(default)]
    pub sounds: HashMap<Id, Sound>,
}

#[derive(Deserialize, Default)]
pub struct State {
    /// Name of the state, does not have to be unique.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub speech: String,
    #[serde(default)]
    pub lights: Lighting,
    /// Ringing time in seconds
    #[serde(default)]
    pub ring: f64,
    #[serde(default)]
    pub terminal: bool,
    #[serde(default)]
    pub sounds: Vec<Id>,
}

#[derive(Deserialize, Default)]
pub struct Sound {
    #[serde(default)]
    pub speech: Option<String>,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub volume: f32,
    /// When the sound is played again after being
    /// interrupted, do not start over but play from
    /// the last playback position minus the specified
    /// time in seconds.
    ///
    /// If the sound was never played before, or if
    /// it has been played in full the last time,
    /// the sound will start over.
    #[serde(default)]
    pub backoff: Option<f64>,
    #[serde(default, rename = "loop")]
    pub looping: bool,
    /// Offset on first playback in seconds.
    pub start_offset: Option<f64>,
}

#[derive(Deserialize, Default)]
pub struct Lighting {
    #[serde(default)]
    pub power: i8,
    #[serde(default)]
    pub excitement: i8,
    #[serde(default)]
    pub mood: i8,
}

#[derive(Deserialize, Default)]
pub struct Transitions {
    /// When input in some format was received.
    #[serde(default)]
    pub dial: HashMap<String, Id>,
    pub pick_up: Option<Id>,
    pub hang_up: Option<Id>,
    /// When all actuators are done.
    pub end: Option<Id>,
    pub timeout: Option<Timeout>,
}

#[derive(Deserialize, Clone)]
pub struct Timeout {
    /// Time in seconds.
    pub after: f64,
    pub to: Id,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn deserialize_example_book() {
        let _book: Book = from_str(include_str!("../../test/testbook_full.yaml"))
            .expect("Expected the example book to work");
    }

    #[test]
    fn deserialize_default_book() {
        let _book: Book = from_str(include_str!("../../resources/demo.yaml"))
            .expect("Expected the default book to work");
    }

    #[test]
    fn deserialize_without_initial_and_transitions() {
        let _book: Book = from_str(include_str!("../../test/testbook_only_states.yaml"))
            .expect("Could not deserialize");
    }

    #[should_panic]
    #[test]
    fn deserialize_empty_should_fail() {
        let _book: Book = from_str("").expect("Could not deserialize");
    }
}
