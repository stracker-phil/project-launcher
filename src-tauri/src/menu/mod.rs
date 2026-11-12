//! The menu bar (Launcher, File, Edit, View, Window, Help). Every custom item's id is the command it runs;
//! `page:<id>` items run the main window's own commands. The standard items (About, Services, Hide, the Edit
//! items, Minimize, Enter Full Screen) are macOS's own. Plain-letter keys stay off the menu items, so they
//! never take a typed letter from a text field.

mod build;
mod items;
pub mod keys;
pub mod palette;
pub mod project_items;
pub mod shortcuts;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

use crate::bus;
use crate::lock::MutexExt;
use crate::state::AppState;
use crate::store::Shortcuts;

pub use build::build;
pub use items::*;

/// Whether the menu items' keys are off while Settings → Shortcuts records one.
#[derive(Default)]
pub struct MenuKeys {
    paused: AtomicBool,
    /// The keys and pause state the shown menu bar was built with; held during a rebuild, so one runs at a
    /// time and the last one reads the latest keys.
    built: Mutex<Option<(Shortcuts, bool)>>,
}

impl MenuKeys {
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

/// Builds the menu bar and puts it up; once, at startup.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let keys = app.state::<MenuKeys>();
    let mut built = keys.built.lock_unpoisoned();
    let now = built_with(app, &keys);
    app.set_menu(build(app)?)?;
    *built = Some(now);
    Ok(())
}

/// Builds the menu bar again when its keys changed. Other settings leave it alone: a rebuild makes macOS
/// redraw every menu title.
pub fn refresh(app: &AppHandle) {
    let keys = app.state::<MenuKeys>();
    let mut built = keys.built.lock_unpoisoned();
    let now = built_with(app, &keys);
    if built.as_ref() == Some(&now) {
        return;
    }
    match build(app).and_then(|menu| app.set_menu(menu)) {
        Ok(_) => *built = Some(now),
        Err(e) => log::warn!("Couldn't rebuild the menu bar: {e}"),
    }
    crate::global_shortcut::apply(app);
}

fn built_with(app: &AppHandle, keys: &MenuKeys) -> (Shortcuts, bool) {
    let shortcuts = app.state::<AppState>().data.settings().shortcuts;
    (shortcuts, keys.paused())
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
