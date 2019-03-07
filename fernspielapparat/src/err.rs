use failure::{Error, Fail, bail};

/// Consumes the given iterator of results and returns
/// `Ok(())` if no errors were encountered.
///
/// Otherwise returns an error describing the whole of
/// the found errors.
pub fn compound_result<I, E, O>(results: I) -> Result<(), Error>
where I : IntoIterator<Item = Result<O, E>>,
    E : Into<Error> {

    let errs = results.into_iter()
        .filter_map(Result::err);

    compound_error(errs)
}

/// Consumes the given iterator of fails or errors and
/// returns `Ok(())` if no errors were encountered.
///
/// Otherwise returns an error describing the whole of
/// the found errors.
pub fn compound_error<I, E>(errors: I) -> Result<(), Error>
    where I : IntoIterator<Item = E>, E : Into<Error> {

    let mut errors = errors.into_iter()
        .map(Into::into);

    match errors.next() {
        None => Ok(()),
        Some(first) => {
            let mut tail : Vec<Error> = errors.collect();
            if tail.is_empty() {
                Err(first)
            } else {
                tail.insert(0, first);
                bail!("Multiple errors: {:?}", tail)
            }
        }
    }
}
