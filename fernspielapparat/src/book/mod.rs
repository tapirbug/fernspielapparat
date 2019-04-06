pub mod book;
mod compile;
use failure::Error;
use std::path::PathBuf;
use crate::state::State;
use serde_yaml;

pub fn from_path<P : Into<PathBuf>>(source_file: P) -> Result<Vec<State>, Error> {
    file::load(source_file)
        .and_then(compile::compile)
}

pub fn from_str<S : AsRef<str>>(source_string: S) -> Result<Vec<State>, Error> {
    let book = serde_yaml::from_str(source_string.as_ref())?;
    compile::compile(book)
}

mod file {
    use super::book::Book;
    use failure::Error;
    use std::path::PathBuf;
    use std::fs::File;
    use serde_yaml::from_reader;

    pub fn load<P : Into<PathBuf>>(source_file: P) -> Result<Book, Error> {
        let mut source_file = File::open(source_file.into())?;
        let book = from_reader(&mut source_file)?;
        Ok(book)
    }
}