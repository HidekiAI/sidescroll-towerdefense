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
    /// The layer this failure belongs to, so `journal_line` can render `layer=` from the
    /// message itself instead of a parameter its callers have no way to know. `None` for
    /// whole-stack rules, which render `-` exactly as a missing field or value does.
    ///
    /// Deliberately NOT `#[serde(default)]`: serde's missing-field path returns `None` for any
    /// `Option`, so a payload written before this field existed already parses without one.
    /// Verified by mutation on 2026-10-18 -- deleting a `#[serde(default)]` from here changed no
    /// test result, so the attribute would be decoration that reads as a requirement. The wire
    /// spelling is what actually needs guarding; see
    /// `validation_message_layer_is_optional_and_spelled_layer`.
    pub layer: Option<String>,
}

impl ValidationMessage {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
            field: None,
            value: None,
            layer: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
            field: None,
            value: None,
            layer: None,
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

    /// Tag the layer this failure belongs to. Per-layer rules in `Stack::validate` call it so
    /// `journal_line` can name the layer without the caller having to know it.
    pub fn with_layer(mut self, layer: impl Into<String>) -> Self {
        self.layer = Some(layer.into());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_valid() {
        let r = ValidationResult::valid();
        assert!(r.is_valid());
        assert!(!r.has_errors());
        assert!(r.messages.is_empty());
    }

    #[test]
    fn test_validation_result_with_error() {
        let r = ValidationResult::valid().with_message(ValidationMessage::error("E1", "error one"));
        assert!(!r.is_valid());
        assert!(r.has_errors());
    }

    #[test]
    fn test_validation_result_with_warning_only() {
        let r =
            ValidationResult::valid().with_message(ValidationMessage::warning("W1", "warning one"));
        assert!(r.is_valid());
        assert!(!r.has_errors());
    }

    #[test]
    fn test_validation_result_merge() {
        let a = ValidationResult::valid().with_message(ValidationMessage::error("E1", "err a"));
        let b = ValidationResult::valid().with_message(ValidationMessage::warning("W1", "warn b"));
        let merged = a.merge(b);
        assert_eq!(merged.messages.len(), 2);
        assert!(!merged.is_valid());
    }

    #[test]
    fn test_validation_result_messages_empty() {
        let r = ValidationResult::valid();
        assert_eq!(r.errors().count(), 0);
    }

    #[test]
    fn test_validation_message_builder() {
        let msg = ValidationMessage::error("CODE", "msg text")
            .with_field("field_name")
            .with_value("val");
        assert_eq!(msg.code, "CODE");
        assert_eq!(msg.message, "msg text");
        assert_eq!(msg.field, Some("field_name".into()));
        assert_eq!(msg.value, Some("val".into()));
        assert_eq!(msg.severity, ValidationSeverity::Error);
    }

    #[test]
    fn test_validation_message_warning_severity() {
        let msg = ValidationMessage::warning("WARN", "warning");
        assert_eq!(msg.severity, ValidationSeverity::Warning);
    }

    /// The wire contract for the additive `layer` field, in the two directions that matter.
    ///
    /// (1) Back-compat: a payload written before the field existed carries no `layer` key and
    /// still parses. This half cannot fail by removing an attribute -- serde's missing-field
    /// path yields `None` for any `Option`, which is why this struct carries no
    /// `#[serde(default)]` (mutation 2026-10-18). It states the contract; it does not prove a
    /// line of code.
    ///
    /// (2) The key is spelled `layer` on the wire, which IS killable: renaming the Rust field
    /// moves the writer and the reader together, so `validation_message_round_trips_the_layer`
    /// would stay green through the rename. Only asserting the literal catches it.
    #[test]
    fn validation_message_layer_is_optional_and_spelled_layer() {
        let legacy = r#"{"severity":"Error","code":"parallax_depth","message":"Sky depth must be > -1","field":"parallax_depth","value":"-1"}"#;
        let decoded: ValidationMessage =
            serde_json::from_str(legacy).expect("legacy payload parses");
        assert_eq!(decoded.layer, None);
        assert_eq!(decoded.code, "parallax_depth");

        let tagged = ValidationMessage::error("plane_depth", "Sky is Back").with_layer("Sky");
        let wire = serde_json::to_string(&tagged).expect("encodes");
        assert!(
            wire.contains("\"layer\":\"Sky\""),
            "wire payload was {wire}"
        );
    }

    /// The decode half: the key a rename would move is read back into the field, so the bridge
    /// can hand the layer to GDScript.
    #[test]
    fn validation_message_round_trips_the_layer() {
        let msg = ValidationMessage::error("plane_depth", "Sky is Back")
            .with_field("plane_depth")
            .with_value("0.5")
            .with_layer("Sky");
        let encoded = serde_json::to_string(&msg).expect("encodes");
        let decoded: ValidationMessage = serde_json::from_str(&encoded).expect("decodes");
        assert_eq!(decoded.layer, Some("Sky".into()));
    }

    #[test]
    fn test_storage_error_display_io() {
        let err = StorageError::Io("file not found".into());
        assert_eq!(format!("{}", err), "IO error: file not found");
    }

    #[test]
    fn test_storage_error_display_parse() {
        let err = StorageError::Parse("bad json".into());
        assert_eq!(format!("{}", err), "Parse error: bad json");
    }

    #[test]
    fn test_storage_error_display_unsupported_version() {
        let err = StorageError::UnsupportedVersion {
            found: "0.2.0".into(),
            expected: "0.1.0".into(),
        };
        assert_eq!(
            format!("{}", err),
            "Unsupported version 0.2.0 (expected 0.1.0)"
        );
    }

    #[test]
    fn test_storage_error_display_missing_field() {
        let err = StorageError::MissingField("tiles".into());
        assert_eq!(format!("{}", err), "Missing required field: tiles");
    }

    #[test]
    fn test_storage_error_display_validation() {
        let vr = ValidationResult::valid()
            .with_message(ValidationMessage::error("E1", "first error"))
            .with_message(ValidationMessage::error("E2", "second error"));
        let err = StorageError::Validation(vr);
        let display = format!("{}", err);
        assert!(display.contains("first error"));
        assert!(display.contains("second error"));
    }
}
