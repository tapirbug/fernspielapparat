pub mod book;
mod compile;
use crate::states::State;
use failure::Error;
use serde_yaml;
use std::path::PathBuf;

pub fn from_path<P: Into<PathBuf>>(source_file: P) -> Result<Vec<State>, Error> {
    file::load(source_file).and_then(compile::compile)
}

pub fn from_str<S: AsRef<str>>(source_string: S) -> Result<Vec<State>, Error> {
    let book = serde_yaml::from_str(source_string.as_ref())?;
    compile::compile(book)
}

mod file {
    use super::book::Book;
    use failure::Error;
    use serde_yaml::from_reader;
    use std::fs::File;
    use std::path::PathBuf;

    pub fn load<P: Into<PathBuf>>(source_file: P) -> Result<Book, Error> {
        let mut source_file = File::open(source_file.into())?;
        let book = from_reader(&mut source_file)?;
        Ok(book)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_compile_default() {
        let states = from_str(include_str!("../../resources/demo.yaml")).unwrap();

        assert_eq!(states[0].name(), "ring");
    }

    #[test]
    fn can_compile_example() {
        let states = from_path("test/testbook_full.yaml").unwrap();

        assert_eq!(states[0].name(), "announcement");
    }

    #[test]
    fn can_compile_generated() {
        let states = from_path("test/testbook_generated.yaml").unwrap();

        assert_eq!(states[0].name(), "ring");
    }
}
