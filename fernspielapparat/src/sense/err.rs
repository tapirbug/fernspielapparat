use failure;

pub enum Error {
    WouldBlock,
    #[allow(dead_code)]
    Fatal(failure::Error),
}

impl Error {
    pub fn fatal<E : Into<failure::Error>>(cause: E) -> Self {
        Error::fatal(cause.into())
    }
}