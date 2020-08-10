use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct StateTypeError {
    state: String,
    line: u32,
}

impl fmt::Display for StateTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StateTypeError: unknown state '{}' at line {}",
            self.state, self.line
        )
    }
}

impl Error for StateTypeError {}

impl StateTypeError {
    pub fn new(state: String, line: u32) -> StateTypeError {
        StateTypeError { state, line }
    }
}
