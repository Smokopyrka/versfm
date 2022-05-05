//! Module that defines view errors
use std::fmt::{self};

/// Error struct used in components
#[derive(Debug, Clone)]
pub struct ComponentError {
    component: String,
    code: String,
    message: String,
}

impl ComponentError {
    pub fn new(component: String, message: String, code: String) -> ComponentError {
        ComponentError {
            component,
            message,
            code,
        }
    }

    pub fn component(&self) -> &str {
        &self.component
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
            "ComponentError: {{ Component: {}, Message: {}, Code: {:?} }}",
            &self.component, &self.message, &self.code
        )
    }
}
