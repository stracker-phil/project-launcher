//! What the owner can change, each checked against the current data and turned into one undoable op.

mod projects;
mod settings;
mod tags;

pub use projects::MetaEdit;

use super::Edit;
use super::ops::Op;
use crate::error::{AppError, AppResult};

fn edit(label: &'static str, op: Op) -> AppResult<Option<Edit>> {
    Ok(Some(Edit { label, op }))
}

/// The trimmed text, or `None` when nothing is left.
fn non_blank(s: &str) -> Option<String> {
    Some(s.trim()).filter(|s| !s.is_empty()).map(str::to_string)
}

/// The trimmed text, or the error from `missing` when nothing is left.
fn required(raw: &str, missing: impl FnOnce() -> AppError) -> AppResult<String> {
    non_blank(raw).ok_or_else(missing)
}

fn invalid_field(field: &str, message: impl Into<String>) -> AppError {
    AppError::InvalidField {
        field: field.into(),
        message: message.into(),
    }
}
