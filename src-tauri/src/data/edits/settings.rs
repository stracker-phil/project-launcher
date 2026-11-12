//! Appearance, accent, app and shortcut edits.

use super::{edit, invalid_field, required};
use crate::data::ops::Op;
use crate::data::{Changed, Data};
use crate::error::{AppError, AppResult};
use crate::store::{Accent, AppSetting, Appearance, DockerManager};

impl Data {
    pub fn set_appearance(&self, appearance: Appearance) -> AppResult<Changed> {
        self.change(|data| {
            if data.settings().appearance == appearance {
                return Ok(None);
            }
            edit("Change Appearance", Op::SetAppearance(appearance))
        })
    }

    pub fn set_accent(&self, accent: Accent) -> AppResult<Changed> {
        self.change(|data| {
            if data.settings().accent == accent {
                return Ok(None);
            }
            edit("Change Accent Color", Op::SetAccent(accent))
        })
    }

    /// Names the app a setting opens with; `checked_app` refuses what it can't use.
    pub fn set_app(&self, setting: AppSetting, value: &str) -> AppResult<Changed> {
        let value = checked_app(setting, value)?;
        self.change(|data| {
            if *setting.of(&mut data.settings()) == value {
                return Ok(None);
            }
            edit(setting.label(), Op::SetApp(setting, value))
        })
    }

    pub fn set_docker_manager(&self, manager: DockerManager) -> AppResult<Changed> {
        self.change(|data| {
            if data.settings().docker_manager == manager {
                return Ok(None);
            }
            edit("Change Docker App", Op::SetDockerManager(manager))
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

    /// Gives `command` the key and leaves `from`, which had it, without one; one undo step.
    pub fn reassign_shortcut(&self, command: &str, key: &str, from: &str) -> AppResult<Changed> {
        let command = command_id(command)?;
        let from = command_id(from)?;
        let key = required(key, || invalid_field("shortcut", "A shortcut needs a key"))?;
        if command == from {
            return self.set_shortcut(&command, Some(key));
        }
        self.change(|_| {
            edit(
                "Reassign Shortcut",
                Op::Batch(vec![
                    Op::SetShortcut {
                        command: from,
                        key: None,
                    },
                    Op::SetShortcut {
                        command,
                        key: Some(key),
                    },
                ]),
            )
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

    /// Back to the command's default key, leaving `from`, which had that key, without one; one undo step.
    pub fn reset_shortcut_taking_from(&self, command: &str, from: &str) -> AppResult<Changed> {
        let command = command_id(command)?;
        let from = command_id(from)?;
        if command == from {
            return self.reset_shortcut(&command);
        }
        self.change(|_| {
            edit(
                "Reset Shortcut",
                Op::Batch(vec![
                    Op::SetShortcut {
                        command: from,
                        key: None,
                    },
                    Op::ResetShortcut { command },
                ]),
            )
        })
    }
}

/// The trimmed value, or the field error (`settings.<key>`) Settings shows below the field.
fn checked_app(setting: AppSetting, raw: &str) -> AppResult<String> {
    let value = raw.trim();
    let error = |message: &str| invalid_field(&format!("settings.{}", setting.key()), message);
    if value.contains(['\n', '\r']) {
        return Err(error("Use one line"));
    }
    if value.is_empty() && setting == AppSetting::TerminalApp {
        return Err(error("Name a terminal app, such as Terminal"));
    }
    Ok(value.to_string())
}

fn command_id(raw: &str) -> AppResult<String> {
    required(raw, || {
        AppError::InvalidInput("A shortcut needs a command".into())
    })
}
