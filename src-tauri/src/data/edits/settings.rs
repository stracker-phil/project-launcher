//! Appearance and shortcut edits.

use super::{edit, invalid_field, required};
use crate::data::ops::Op;
use crate::data::{Changed, Data};
use crate::error::{AppError, AppResult};
use crate::store::Appearance;

impl Data {
    pub fn set_appearance(&self, appearance: Appearance) -> AppResult<Changed> {
        self.change(|data| {
            if data.settings().appearance == appearance {
                return Ok(None);
            }
            edit("Change Appearance", Op::SetAppearance(appearance))
        })
    }

    /// Sets the owner's key for a command; `None` leaves the command without one.
    pub fn set_shortcut(&self, command: &str, key: Option<String>) -> AppResult<Changed> {
        let command = command_id(command)?;
        let key = key
            .map(|key| required(&key, || invalid_field("shortcut", "A shortcut needs a key")))
            .transpose()?;
        self.change(|data| {
            if data.settings().shortcuts.get(&command) == Some(&key) {
                return Ok(None);
            }
            edit("Change Shortcut", Op::SetShortcut { command, key })
        })
    }

    /// Back to the command's default key.
    pub fn reset_shortcut(&self, command: &str) -> AppResult<Changed> {
        let command = command_id(command)?;
        self.change(|data| {
            if !data.settings().shortcuts.contains_key(&command) {
                return Ok(None);
            }
            edit("Reset Shortcut", Op::ResetShortcut { command })
        })
    }
}

fn command_id(raw: &str) -> AppResult<String> {
    required(raw, || AppError::InvalidInput("A shortcut needs a command".into()))
}
