use std::fmt::{self};

#[derive(Debug, Clone)]
pub struct ComponentError {
    code: String,
    message: String,
}

impl ComponentError {
    pub fn new(message: String, code: String) -> ComponentError {
        ComponentError { message, code }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ComponentError: {{ Message: {}, Code: {:?} }}",
            &self.message, &self.code
        )
    }
}
