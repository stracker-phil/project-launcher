//! The menu bar's Project menu follows the project the main window shows: an item that can't run for it, or
//! every item while none is shown, is disabled in place, without rebuilding the bar.

use tauri::menu::MenuItemKind;
use tauri::{AppHandle, Manager, Wry};

use super::PROJECT;
use super::palette::project_state;
use super::project_items::{ProjectState, applies};
use crate::bus::{self, CommandStatus};
use crate::state::AppState;

fn enabled(command: &str, commands: &dyn CommandStatus, project: Option<&ProjectState>) -> bool {
    commands.is_enabled(command) && project.is_some_and(|project| applies(command, project))
}

fn project_commands() -> impl Iterator<Item = &'static str> {
    PROJECT
        .iter()
        .flat_map(|items| items.iter())
        .map(|item| item.command)
}

fn is_project_item(id: &str) -> bool {
    project_commands().any(|command| command == id)
}

fn apply_to(
    items: Vec<MenuItemKind<Wry>>,
    commands: &dyn CommandStatus,
    project: Option<&ProjectState>,
) -> tauri::Result<()> {
    for item in items {
        match &item {
            MenuItemKind::Submenu(submenu) => apply_to(submenu.items()?, commands, project)?,
            MenuItemKind::MenuItem(menu_item) if is_project_item(menu_item.id().as_ref()) => {
                menu_item.set_enabled(enabled(menu_item.id().as_ref(), commands, project))?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn apply(app: &AppHandle) {
    let Some(menu) = app.menu() else {
        return;
    };
    let state = app.state::<AppState>();
    let project = state
        .current_project()
        .and_then(|id| project_state(&state, &id));
    let registry = app.state::<bus::Commands>();
    let commands = registry.available(app);
    if let Err(e) = menu
        .items()
        .and_then(|items| apply_to(items, &commands, project.as_ref()))
    {
        log::warn!("Couldn't update the Project menu: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Enabled(Vec<&'static str>);

    impl CommandStatus for Enabled {
        fn knows(&self, id: &str) -> bool {
            self.0.contains(&id)
        }

        fn is_enabled(&self, id: &str) -> bool {
            self.knows(id)
        }
    }

    fn project_commands() -> Vec<&'static str> {
        PROJECT
            .iter()
            .flat_map(|items| items.iter())
            .map(|item| item.command)
            .collect()
    }

    fn state() -> ProjectState {
        ProjectState {
            available: true,
            locked: false,
            starred: false,
            archived: false,
            has_database: true,
            can_manage_claude: true,
            tag: None,
            tags: Vec::new(),
        }
    }

    #[test]
    fn every_project_item_is_enabled_for_a_project_that_allows_it() {
        let bus = Enabled(project_commands());

        for command in project_commands() {
            assert!(enabled(command, &bus, Some(&state())), "{command}");
        }
    }

    #[test]
    fn every_project_item_is_disabled_while_no_project_is_shown() {
        let bus = Enabled(project_commands());

        for command in project_commands() {
            assert!(!enabled(command, &bus, None), "{command}");
        }
    }

    #[test]
    fn a_locked_project_disables_rename_icon_and_notes_only() {
        let bus = Enabled(project_commands());
        let locked = ProjectState {
            locked: true,
            ..state()
        };

        for command in project_commands() {
            let edits = matches!(
                command,
                "page:project.rename" | "page:project.icon" | "page:project.notes"
            );
            assert_eq!(enabled(command, &bus, Some(&locked)), !edits, "{command}");
        }
    }

    #[test]
    fn a_project_without_a_database_disables_open_database_only() {
        let bus = Enabled(project_commands());
        let project = ProjectState {
            has_database: false,
            ..state()
        };

        for command in project_commands() {
            assert_eq!(
                enabled(command, &bus, Some(&project)),
                command != "page:project.database",
                "{command}"
            );
        }
    }

    #[test]
    fn a_project_claude_manager_cannot_open_disables_manage_claude_only() {
        let bus = Enabled(project_commands());
        let project = ProjectState {
            can_manage_claude: false,
            ..state()
        };

        for command in project_commands() {
            assert_eq!(
                enabled(command, &bus, Some(&project)),
                command != "page:project.claude",
                "{command}"
            );
        }
    }

    #[test]
    fn a_command_the_bus_does_not_enable_stays_disabled() {
        let bus = Enabled(Vec::new());

        for command in project_commands() {
            assert!(!enabled(command, &bus, Some(&state())), "{command}");
        }
    }
}
