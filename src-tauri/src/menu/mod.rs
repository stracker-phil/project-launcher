//! The menu bar (Launcher, File, Edit, View, Window, Help). Every custom item's id is the command it runs;
//! `page:<id>` items run the main window's own commands. The standard items (About, Services, Hide, the Edit
//! items, Minimize, Enter Full Screen) are macOS's own. Plain-letter keys stay off the menu items, so they
//! never take a typed letter from a text field.

mod build;
mod items;

use tauri::{AppHandle, Manager};

use crate::bus;

pub use build::build;
pub use items::*;

/// Runs the command a clicked item names; ids the bus doesn't know are ignored.
pub fn clicked(app: &AppHandle, id: &str) {
    if app.state::<bus::Commands>().knows(id) {
        bus::spawn(app, id);
    }
}
