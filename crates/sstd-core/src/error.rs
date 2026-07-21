use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ValidationMessage {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub value: Option<String>,
}

impl ValidationMessage {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
            field: None,
            value: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
            field: None,
            value: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ValidationResult {
    pub messages: Vec<ValidationMessage>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.messages
            .iter()
            .all(|m| m.severity != ValidationSeverity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == ValidationSeverity::Error)
    }

    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }

    pub fn with_message(mut self, msg: ValidationMessage) -> Self {
        self.messages.push(msg);
        self
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.messages.extend(other.messages);
        self
    }
}

#[derive(Debug, Clone)]
pub enum StorageError {
    Io(String),
    Parse(String),
    UnsupportedVersion { found: String, expected: String },
    MissingField(String),
    Validation(ValidationResult),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(msg) => write!(f, "IO error: {}", msg),
            StorageError::Parse(msg) => write!(f, "Parse error: {}", msg),
            StorageError::UnsupportedVersion { found, expected } => {
                write!(f, "Unsupported version {} (expected {})", found, expected)
            }
            StorageError::MissingField(field) => write!(f, "Missing required field: {}", field),
            StorageError::Validation(r) => {
                let errors: Vec<_> = r.errors().map(|m| m.message.as_str()).collect();
                write!(f, "Validation failed: {}", errors.join(", "))
            }
        }
    }
}

pub type SstdResult<T> = Result<T, StorageError>;
