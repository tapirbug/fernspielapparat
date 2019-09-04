mod compile;
pub(crate) mod spec;
pub use compile::{compile, Book};
use failure::Error;
use serde_yaml;
use std::path::Path;

pub fn from_path(source_file: impl AsRef<Path>) -> Result<Book, Error> {
    file::load(source_file).and_then(compile)
}

pub fn from_str(source_string: impl AsRef<str>) -> Result<Book, Error> {
    let book = serde_yaml::from_str(source_string.as_ref())?;
    compile(book)
}

/// pub(crate) for testing, loads YAML files
pub(crate) mod file {
    use super::spec;
    use failure::Error;
    use serde_yaml::from_reader;
    use std::fs::File;
    use std::path::Path;

    pub fn load<P: AsRef<Path>>(source_file: P) -> Result<spec::Book, Error> {
        let mut source_file = File::open(source_file.as_ref())?;
        let book = from_reader(&mut source_file)?;
        Ok(book)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::senses::Input;

    #[test]
    fn can_compile_default() {
        let book = from_str(include_str!("../../resources/demo.yaml")).unwrap();
        let states = book.states();

        assert_eq!(states[0].name(), "ring");
    }

    #[test]
    fn can_compile_example() {
        let book = from_path("test/testbook_full.yaml").unwrap();
        let states = book.states();

        assert_eq!(states[0].name(), "announcement");
    }

    #[test]
    fn can_compile_generated() {
        let book = from_path("test/testbook_generated.yaml").unwrap();
        let states = book.states();
        let state_with_dial_1_transition = states.iter().find(|s| s.name() == "Speak").unwrap();

        let has_transition_for_1 = state_with_dial_1_transition
            .transition_for_input(Input::digit(1).unwrap())
            .is_some();

        assert!(
            has_transition_for_1,
            "Expected transition for input 1 to be defined"
        )
    }
}
