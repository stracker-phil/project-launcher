//! The menu bar (Launcher, File, Edit, View, Window, Help). Every custom item's id is the command it runs;
//! `page:<id>` items run the main window's own commands. The standard items (About, Services, Hide, the Edit
//! items, Minimize, Enter Full Screen) are macOS's own. Plain-letter keys stay off the menu items, so they
//! never take a typed letter from a text field.

mod build;
mod items;
pub mod keys;
pub mod palette;
pub mod shortcuts;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

use crate::bus;
use crate::lock::MutexExt;

pub use build::build;
pub use items::*;

/// Whether the menu items' keys are off while Settings → Shortcuts records one.
#[derive(Default)]
pub struct MenuKeys {
    paused: AtomicBool,
    /// One rebuild at a time, so the last one reads the latest keys and is the one shown.
    rebuilding: Mutex<()>,
}

impl MenuKeys {
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

/// Builds the menu bar again, e.g. after the owner's keys changed.
pub fn refresh(app: &AppHandle) {
    let keys = app.state::<MenuKeys>();
    let _rebuilding = keys.rebuilding.lock_unpoisoned();
    if let Err(e) = build(app).and_then(|menu| app.set_menu(menu)) {
        log::warn!("Couldn't rebuild the menu bar: {e}");
    }
}

/// Takes the menu items' keys off while a key is being recorded, and puts them back after.
#[tauri::command]
pub async fn pause_menu_keys(app: AppHandle, paused: bool) {
    let keys = app.state::<MenuKeys>();
    if keys.paused.swap(paused, Ordering::Relaxed) != paused {
        refresh(&app);
    }
}

/// Runs the command a clicked item names; ids the bus doesn't know are ignored.
pub fn clicked(app: &AppHandle, id: &str) {
    if app.state::<bus::Commands>().knows(id) {
        bus::spawn(app, id);
    }
}
