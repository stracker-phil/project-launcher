//! The app's own commands; features add theirs with their own `register`.

use tauri::AppHandle;

use super::{Command, CommandBus};
use crate::data::edit_menu::{self, Revert};
use crate::{folders, window};

type Bus = CommandBus<AppHandle>;

pub fn register(bus: &mut Bus) {
    bus.add(
        Command::new("app:show", "Open Launcher")
            .help("Show the Launcher window")
            .run_now(window::show_main),
    );
    bus.add(
        Command::new("app:settings", "Settings…")
            .help("Open Settings")
            .run_now(|app: &AppHandle| window::settings::open(app, None)),
    );
    bus.add_family(
        Command::new("app:settings:", "Settings Page")
            .help("Open Settings at a page")
            .hidden()
            .family("<page>")
            .run(|app, page| async move {
                window::settings::open(&app, Some(&page));
                Ok(())
            }),
    );
    bus.add_family(
        Command::new("page:", "Window Action")
            .help("Run one of the main window's own commands (the menus' items for what the page does)")
            .hidden()
            .family("<command>")
            .run(|app, id| async move {
                window::run_page_command(&app, &id);
                Ok(())
            }),
    );
    bus.add(
        Command::new("window:close", "Close Window")
            .help("Hide the window in front")
            .hidden()
            .run_now(window::close_focused),
    );
    bus.add(
        Command::new("edit:undo", "Undo")
            .help("Undo the last change (in a text field: the typing)")
            .run(|app| async move { edit_menu::request(&app, Revert::Undo) }),
    );
    bus.add(
        Command::new("edit:redo", "Redo")
            .help("Redo the last undone change (in a text field: the typing)")
            .run(|app| async move { edit_menu::request(&app, Revert::Redo) }),
    );
    bus.add(
        Command::new("app:logs", "Open Log Folder")
            .run(|app| async move { folders::open_log_folder(&app) }),
    );
    bus.add(
        Command::new("app:data", "Show Data Folder")
            .help("Show the folder with the settings and projects list in Finder")
            .run(|app| async move { folders::open_data_folder(&app) }),
    );
    bus.add(
        Command::new("app:quit", "Quit Launcher")
            .quits()
            .run_now(|app: &AppHandle| app.exit(0)),
    );
    #[cfg(debug_assertions)]
    bus.add(
        Command::new("app:reload", "Reload")
            .help("Reload the focused window's page")
            .run_now(window::reload_focused),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered() -> Bus {
        let mut bus = Bus::default();
        register(&mut bus);
        bus
    }

    #[test]
    fn the_app_commands_have_their_titles_and_visibility() {
        let bus = registered();

        for (id, title, hidden, quits) in [
            ("app:show", "Open Launcher", false, false),
            ("app:quit", "Quit Launcher", false, true),
            ("app:logs", "Open Log Folder", false, false),
            ("app:data", "Show Data Folder", false, false),
            ("window:close", "Close Window", true, false),
            ("edit:undo", "Undo", false, false),
            ("edit:redo", "Redo", false, false),
        ] {
            let info = bus.info(id).unwrap_or_else(|| panic!("{id} is not registered"));
            assert_eq!(info.title, title, "{id}");
            assert_eq!(info.hidden, hidden, "{id}");
            assert_eq!(info.quits, quits, "{id}");
        }
    }

    #[test]
    fn show_data_folder_explains_what_it_opens() {
        let bus = registered();
        assert_eq!(
            bus.info("app:data").unwrap().help,
            "Show the folder with the settings and projects list in Finder"
        );
    }

    #[test]
    fn page_command_ids_resolve_to_the_hidden_window_action_family() {
        let bus = registered();

        for id in ["page:view.toggleSidebar", "page:palette.open"] {
            assert!(bus.knows(id), "{id}");
            let info = bus.info(id).unwrap();
            assert_eq!(info.title, "Window Action", "{id}");
            assert!(info.hidden, "{id}");
        }
        assert_eq!(bus.family("page:").unwrap().arg, "<command>");
    }

    #[test]
    fn reload_is_registered_in_debug_builds_only() {
        assert_eq!(registered().knows("app:reload"), cfg!(debug_assertions));
    }

    #[test]
    fn settings_opens_from_its_own_command() {
        let bus = registered();
        let info = bus.info("app:settings").unwrap();
        assert_eq!(info.title, "Settings…");
        assert_eq!(info.help, "Open Settings");
        assert!(!info.hidden);
    }

    #[test]
    fn settings_page_ids_resolve_to_the_hidden_settings_page_family() {
        let bus = registered();

        let id = "app:settings:about";
        assert!(bus.knows(id));
        let info = bus.info(id).unwrap();
        assert_eq!(info.title, "Settings Page");
        assert_eq!(info.help, "Open Settings at a page");
        assert!(info.hidden);
        assert_eq!(bus.family("app:settings:").unwrap().arg, "<page>");
    }
}
