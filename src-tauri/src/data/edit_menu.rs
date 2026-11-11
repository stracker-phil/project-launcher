//! Undo and Redo from the Edit menu. The menu's ⌘Z reaches no text field, so the focused window decides: its
//! page undoes the typing in a focused text field, or else calls `undo_change`. Without a focused window
//! (the tray, the palette panel, the CLI) the data change is undone directly.

use tauri::{AppHandle, Emitter};

use super::commands;
use crate::error::AppResult;
use crate::events::EDIT_REQUEST_EVENT;
use crate::window;

#[derive(Clone, Copy)]
pub enum Revert {
    Undo,
    Redo,
}

pub fn request(app: &AppHandle, revert: Revert) -> AppResult<()> {
    let name = match revert {
        Revert::Undo => "undo",
        Revert::Redo => "redo",
    };
    if let Some(label) = window::focused_regular(app) {
        if let Err(e) = app.emit_to(label.as_str(), EDIT_REQUEST_EVENT, name) {
            log::warn!("Couldn't ask the {label} window to {name}: {e}");
        }
        return Ok(());
    }
    match revert {
        Revert::Undo => commands::undo(app),
        Revert::Redo => commands::redo(app),
    }
    .map(|_| ())
}
