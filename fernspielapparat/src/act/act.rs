use failure::Error;

pub trait Act {
    /// Tries to check if the act has either completed
    /// or been cancelled.
    ///
    /// Returns an error only if the check failed.
    fn done(&self) -> Result<bool, Error>;

    /// Tries to cancel the act, if still running.
    ///
    /// If still running and successfully cancelled,
    /// or already stopped (by itself or by cancel),
    /// returns `Ok(())`. Returns an error only if
    /// cancellation failed.
    fn cancel(&mut self) -> Result<(), Error>;
}
