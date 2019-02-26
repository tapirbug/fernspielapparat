
struct Machine {

}

impl Machine {

    /// Starts the next cycle of the machine, first polling
    /// input and changing state if necessary, then setting
    /// the state of actuators.
    pub fn update(&mut self) -> bool {
        let terminal = self.sense();
        self.actuate();
        terminal
    }

    /// Accepts the next input from actuators and returns
    /// `true`, if still running. On termination, returns
    /// `false`. This operation may change the current state.
    fn sense(&mut self) -> bool {
        unimplemented!()
    }


    fn actuate(&mut self) {

    }

    /// `true`, if a terminal state has been reached.
    pub fn is_terminal(&self) -> bool {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    
}