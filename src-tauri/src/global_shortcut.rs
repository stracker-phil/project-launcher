//! ⌃⌘L shows or hides the main window from anywhere; release builds only, so the debug app never takes the key
//! (ADR 0036). The owner rebinds it in Settings → Shortcuts, stored under `app:toggle` like a menu key.

use tauri::{AppHandle, Manager};

use crate::store::Shortcuts;
use crate::window;

pub const COMMAND: &str = "app:toggle";
pub const LABEL: &str = "Show or Hide Launcher";
/// Settings → Shortcuts lists it in a group of its own.
pub const GROUP: &str = "Global";
pub const DEFAULT_KEY: &str = "Ctrl+Mod+L";

/// The key in effect: the owner's (`None` when they removed it), else the default.
pub fn key(shortcuts: &Shortcuts) -> Option<String> {
    match shortcuts.get(COMMAND) {
        Some(own) => own.clone(),
        None => Some(DEFAULT_KEY.into()),
    }
}

/// Hides the main window while it is in front, else shows it.
pub fn toggle(app: &AppHandle) {
    let in_front = app
        .get_webview_window(window::MAIN)
        .is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false));
    if in_front {
        window::hide_main(app);
    } else {
        window::show_main(app);
    }
}

#[cfg(not(debug_assertions))]
pub use release::{apply, plugin};

/// Debug builds register no global shortcut.
#[cfg(debug_assertions)]
pub fn apply(_: &AppHandle) {}

#[cfg(not(debug_assertions))]
mod release {
    use tauri::plugin::TauriPlugin;
    use tauri::{AppHandle, Manager, Wry};
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

    use crate::menu::MenuKeys;
    use crate::menu::keys::Combo;
    use crate::state::AppState;

    pub fn plugin() -> TauriPlugin<Wry> {
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _, event| {
                if event.state() == ShortcutState::Pressed {
                    super::toggle(app);
                }
            })
            .build()
    }

    /// Registers the key in effect, or none while a key is being recorded; a key another app holds is logged.
    pub fn apply(app: &AppHandle) {
        let shortcuts = app.global_shortcut();
        if let Err(e) = shortcuts.unregister_all() {
            log::warn!("Couldn't release the global shortcut: {e}");
        }
        if app.state::<MenuKeys>().paused() {
            return;
        }
        let key = super::key(&app.state::<AppState>().data.settings().shortcuts);
        let Some(shortcut) = key
            .as_deref()
            .and_then(Combo::parse)
            .and_then(|combo| combo.accelerator().parse::<Shortcut>().ok())
        else {
            return;
        };
        if let Err(e) = shortcuts.register(shortcut) {
            log::warn!("The global shortcut is taken: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(entry: Option<Option<&str>>) -> Shortcuts {
        let mut shortcuts = Shortcuts::new();
        if let Some(key) = entry {
            shortcuts.insert(COMMAND.to_string(), key.map(str::to_string));
        }
        shortcuts
    }

    #[test]
    fn without_an_override_the_default_key_is_in_effect() {
        assert_eq!(key(&with(None)), Some("Ctrl+Mod+L".to_string()));
    }

    #[test]
    fn the_owners_key_replaces_the_default() {
        assert_eq!(
            key(&with(Some(Some("Mod+Shift+J")))),
            Some("Mod+Shift+J".to_string())
        );
    }

    #[test]
    fn a_key_the_owner_removed_leaves_no_global_shortcut() {
        assert_eq!(key(&with(Some(None))), None);
    }

    #[test]
    fn overrides_of_other_commands_do_not_touch_the_global_key() {
        let mut shortcuts = Shortcuts::new();
        shortcuts.insert("app:quit".to_string(), None);
        assert_eq!(key(&shortcuts), Some("Ctrl+Mod+L".to_string()));
    }
}
