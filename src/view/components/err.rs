use std::fmt::{self};

#[derive(Clone, Debug)]
pub enum ComponentErrorKind {
    AlreadyExists,
    ConnectionProblem,
    FileNotFound,
    Incomplete,
    InvalidData,
    InsufficientPermissions,
    IsDirectory,
    IsNotDirectory,
    Unexpected,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct ComponentError {
    kind: ComponentErrorKind,
    message: String,
}

impl ComponentError {
    pub fn new(message: String, kind: ComponentErrorKind) -> ComponentError {
        ComponentError { message, kind }
    }

    pub fn kind(&self) -> &ComponentErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ComponentError: {{ Message: {}, Kind: {:?} }}",
            &self.message, &self.kind
        )
    }
}
