//! The app's error type and its JSON shape across the Tauri boundary (`{ kind, message, field? }`).

use serde::{Deserialize, Serialize, Serializer};
use ts_rs::TS;

pub type AppResult<T> = Result<T, AppError>;

/// Every error that crosses the Tauri boundary. Serialized as `{ kind, message, field? }` so the UI can
/// branch on `kind`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("File error: {0}")]
    Io(String),
    #[error("{0}")]
    InvalidInput(String),
    /// Invalid input in one form field; `field` is its path (`settings.terminalApp`).
    #[error("`{field}`: {message}")]
    InvalidField { field: String, message: String },
    #[error("{0}")]
    NotFound(String),
    /// A bug or a broken invariant; the message is for the log and a bug report.
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            AppError::Io(_) => ErrorKind::Io,
            AppError::InvalidInput(_) | AppError::InvalidField { .. } => ErrorKind::InvalidInput,
            AppError::NotFound(_) => ErrorKind::NotFound,
            AppError::Internal(_) => ErrorKind::Internal,
        }
    }

    pub fn field(&self) -> Option<FieldJson<'_>> {
        match self {
            AppError::InvalidField { field, message } => Some(FieldJson {
                path: field,
                message,
            }),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorJson {
            kind: self.kind(),
            message: self.to_string(),
            field: self.field(),
        }
        .serialize(serializer)
    }
}

/// The JSON shape of an `AppError`.
#[derive(Serialize, TS)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "AppError")]
struct ErrorJson<'a> {
    kind: ErrorKind,
    message: String,
    /// The form field the error is about (`InvalidField` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    field: Option<FieldJson<'a>>,
}

/// `message` is the error's message without the `` `path`: `` prefix.
#[derive(Serialize, TS, Debug, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "AppErrorField")]
pub struct FieldJson<'a> {
    pub path: &'a str,
    pub message: &'a str,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Io,
    InvalidInput,
    NotFound,
    Internal,
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::InvalidInput(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_variant_serializes_kind_and_message() {
        let cases = [
            (
                AppError::Io("denied".into()),
                json!({ "kind": "io", "message": "File error: denied" }),
            ),
            (
                AppError::InvalidInput("Not a web link".into()),
                json!({ "kind": "invalid_input", "message": "Not a web link" }),
            ),
            (
                AppError::NotFound("Project Northwind".into()),
                json!({ "kind": "not_found", "message": "Project Northwind" }),
            ),
            (
                AppError::Internal("broken".into()),
                json!({ "kind": "internal", "message": "broken" }),
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(serde_json::to_value(&err).unwrap(), expected, "{err:?}");
        }
    }

    #[test]
    fn invalid_field_serializes_its_path_and_message_without_prefix() {
        let err = AppError::InvalidField {
            field: "settings.terminalApp".into(),
            message: "must not be empty".into(),
        };
        assert_eq!(
            serde_json::to_value(&err).unwrap(),
            json!({
                "kind": "invalid_input",
                "message": "`settings.terminalApp`: must not be empty",
                "field": { "path": "settings.terminalApp", "message": "must not be empty" }
            })
        );
    }

    #[test]
    fn every_variant_has_its_kind() {
        let cases = [
            (AppError::Io(String::new()), ErrorKind::Io),
            (
                AppError::InvalidInput(String::new()),
                ErrorKind::InvalidInput,
            ),
            (
                AppError::InvalidField {
                    field: "a".into(),
                    message: "b".into(),
                },
                ErrorKind::InvalidInput,
            ),
            (AppError::NotFound(String::new()), ErrorKind::NotFound),
            (AppError::Internal(String::new()), ErrorKind::Internal),
        ];
        for (err, kind) in cases {
            assert_eq!(err.kind(), kind, "{err:?}");
        }
    }

    #[test]
    fn io_and_json_errors_convert_to_their_variants() {
        let io: AppError = std::io::Error::other("disk full").into();
        assert_eq!(io, AppError::Io("disk full".into()));

        let parse = serde_json::from_str::<u32>("x").unwrap_err();
        let expected = AppError::InvalidInput(parse.to_string());
        assert_eq!(AppError::from(parse), expected);
    }
}
