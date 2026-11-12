//! The menu-bar item, the app's home while every window is hidden (ADR 0034): open, the palette, recent
//! projects (⌥1…⌥0), the current project's actions, Settings and Quit. Its items' ids are bus commands; the
//! app's menu event handler (`menu::clicked`) runs them, as it does the menu bar's. `refresh` rebuilds it when
//! what it lists changed.

use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;

use tauri::{AppHandle, Manager, Wry};

use crate::lock::MutexExt;
use crate::menu::keys::Combo;
use crate::menu::{QUIT, SETTINGS};
use crate::projects::Project;
use crate::projects::config::ProjectConfig;
use crate::run::bus;
use crate::state::AppState;

const TRAY_ID: &str = "main";
/// What the shown menu lists; held during a rebuild, so one runs at a time.
static BUILT: Mutex<Option<Contents>> = Mutex::new(None);
/// Recent projects the tray lists, each with ⌥ and its digit (the tenth is ⌥0).
const RECENT: usize = 10;

/// 36 px source for the 18 pt menu bar icon.
const ICON: &[u8] = include_bytes!("../icons/tray/tray.png");

#[derive(Debug, PartialEq)]
pub struct Row {
    pub label: String,
    pub command: String,
    pub accelerator: Option<String>,
}

/// What the tray lists besides its fixed rows.
#[derive(Debug, PartialEq, Default)]
pub struct Contents {
    pub recent: Vec<Row>,
    pub current: Option<(String, Vec<Row>)>,
}

/// The most recently opened projects (not archived, not the current one), newest first, with ⌥1…⌥0.
pub fn recent_rows(projects: &[Project], current: Option<&str>) -> Vec<Row> {
    let mut opened: Vec<&Project> = projects
        .iter()
        .filter(|p| {
            p.last_opened_at.is_some() && !p.meta.archived && Some(p.id.as_str()) != current
        })
        .collect();
    opened.sort_by_key(|p| std::cmp::Reverse(p.last_opened_at));
    opened
        .into_iter()
        .take(RECENT)
        .enumerate()
        .map(|(index, project)| Row {
            label: project.name.clone(),
            command: bus::show_command(&project.id),
            accelerator: Some(format!("Alt+{}", (index + 1) % 10)),
        })
        .collect()
}

/// The current project's apps and scripts with their keys, then Reveal in Finder.
pub fn current_rows(id: &str, config: &ProjectConfig) -> Vec<Row> {
    config
        .apps
        .iter()
        .chain(&config.scripts)
        .map(|action| Row {
            label: action.label.clone(),
            command: bus::launch_command(id, action.edit.source),
            accelerator: action
                .shortcut
                .as_deref()
                .and_then(Combo::parse)
                .map(|combo| combo.accelerator()),
        })
        .chain(std::iter::once(Row {
            label: "Reveal in Finder".into(),
            command: bus::reveal_command(id),
            accelerator: None,
        }))
        .collect()
}

fn contents(app: &AppHandle) -> Contents {
    let state = app.state::<AppState>();
    let current = state.current_project();
    let projects = state.data.projects();
    Contents {
        recent: recent_rows(&projects, current.as_deref()),
        current: current.and_then(|id| {
            let project = projects.iter().find(|p| p.id == id && p.available)?;
            let config = state.data.project_config(&id).ok()?;
            Some((project.name.clone(), current_rows(&id, &config)))
        }),
    }
}

fn item(app: &AppHandle, row: &Row) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(
        app,
        &row.command,
        &row.label,
        true,
        row.accelerator.as_deref(),
    )
}

/// A row of its own: an id, a label and an optional key.
fn fixed(
    app: &AppHandle,
    id: &str,
    label: impl AsRef<str>,
    key: Option<&str>,
) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, label, true, key)
}

fn recent_submenu(app: &AppHandle, rows: &[Row]) -> tauri::Result<Submenu<Wry>> {
    let items: Vec<MenuItem<Wry>> = rows
        .iter()
        .map(|row| item(app, row))
        .collect::<tauri::Result<_>>()?;
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i as &dyn IsMenuItem<Wry>).collect();
    Submenu::with_items(app, "Recent Projects", !items.is_empty(), &refs)
}

/// The project's name as a gray caption, then its rows.
fn current_section(
    app: &AppHandle,
    project: &str,
    rows: &[Row],
) -> tauri::Result<Vec<Box<dyn IsMenuItem<Wry>>>> {
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = vec![
        Box::new(PredefinedMenuItem::separator(app)?),
        Box::new(MenuItem::new(app, project, false, None::<&str>)?),
    ];
    for row in rows {
        items.push(Box::new(item(app, row)?));
    }
    Ok(items)
}

fn build(app: &AppHandle, contents: &Contents) -> tauri::Result<Menu<Wry>> {
    let name = crate::APP_NAME;
    let mut owned: Vec<Box<dyn IsMenuItem<Wry>>> = vec![
        Box::new(fixed(app, "app:show", format!("Open {name}"), None)?),
        Box::new(fixed(app, "app:palette", "Command Palette…", None)?),
        Box::new(PredefinedMenuItem::separator(app)?),
        Box::new(recent_submenu(app, &contents.recent)?),
    ];
    if let Some((project, rows)) = &contents.current {
        owned.extend(current_section(app, project, rows)?);
    }
    owned.push(Box::new(PredefinedMenuItem::separator(app)?));
    owned.push(Box::new(fixed(
        app,
        SETTINGS.command,
        SETTINGS.label,
        SETTINGS.accelerator,
    )?));
    owned.push(Box::new(fixed(
        app,
        QUIT.command,
        format!("Quit {name}"),
        QUIT.accelerator,
    )?));
    let refs: Vec<&dyn IsMenuItem<Wry>> = owned.iter().map(|i| i.as_ref()).collect();
    Menu::with_items(app, &refs)
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let mut built = BUILT.lock_unpoisoned();
    let now = contents(app);
    let menu = build(app, &now)?;
    // No `on_menu_event` here: Tauri hands tray menu clicks to the app's handler too, so they would run twice.
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON)?)
        .icon_as_template(true)
        .tooltip(crate::APP_NAME)
        .menu(&menu)
        .build(app)?;
    *built = Some(now);
    Ok(())
}

/// Rebuilds the menu when what it lists changed.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let mut built = BUILT.lock_unpoisoned();
    let now = contents(app);
    if built.as_ref() == Some(&now) {
        return;
    }
    match build(app, &now) {
        Ok(menu) => match tray.set_menu(Some(menu)) {
            Ok(()) => *built = Some(now),
            Err(e) => log::warn!("Couldn't update the menu-bar item: {e}"),
        },
        Err(e) => log::warn!("Couldn't build the menu-bar item's menu: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::projects::ProjectMeta;
    use crate::projects::config::{Context, resolve};
    use crate::store::Settings;

    fn project(id: &str, name: &str, last_opened_at: Option<i64>, archived: bool) -> Project {
        Project {
            id: id.into(),
            path: format!("~/{id}"),
            name: name.into(),
            meta: ProjectMeta {
                archived,
                ..ProjectMeta::default()
            },
            added_at: 0,
            last_opened_at,
            available: true,
            locked: false,
            file_error: None,
        }
    }

    fn config(file: serde_json::Value) -> ProjectConfig {
        let settings = Settings::default();
        let cx = Context {
            dir: Path::new("/work/northwind"),
            home: Some(Path::new("/Users/demo")),
            settings: &settings,
            is_git_repo: false,
            uses_claude: false,
        };
        resolve(file.as_object().unwrap(), &cx)
    }

    fn row(label: &str, command: &str, accelerator: Option<&str>) -> Row {
        Row {
            label: label.into(),
            command: command.into(),
            accelerator: accelerator.map(str::to_string),
        }
    }

    #[test]
    fn recent_projects_are_listed_newest_first_with_show_commands_and_alt_digits() {
        let projects = [
            project("alpha", "Alpha", Some(100), false),
            project("bravo", "Bravo", Some(300), false),
            project("charlie", "Charlie", Some(200), false),
        ];
        assert_eq!(
            recent_rows(&projects, None),
            [
                row("Bravo", "project:show:bravo", Some("Alt+1")),
                row("Charlie", "project:show:charlie", Some("Alt+2")),
                row("Alpha", "project:show:alpha", Some("Alt+3")),
            ]
        );
    }

    #[test]
    fn recent_projects_leave_out_never_opened_archived_and_current_ones() {
        let projects = [
            project("never-opened", "Never Opened", None, false),
            project("old-news", "Old News", Some(500), true),
            project("acme-website", "Acme Website", Some(400), false),
            project("northwind", "Northwind", Some(300), false),
        ];
        assert_eq!(
            recent_rows(&projects, Some("acme-website")),
            [row("Northwind", "project:show:northwind", Some("Alt+1"))]
        );
    }

    #[test]
    fn recent_projects_are_empty_without_opened_projects() {
        assert_eq!(recent_rows(&[], None), Vec::<Row>::new());
        assert_eq!(
            recent_rows(&[project("alpha", "Alpha", None, false)], None),
            Vec::<Row>::new()
        );
    }

    #[test]
    fn recent_projects_stop_at_ten_and_the_tenth_takes_alt_zero() {
        let projects: Vec<Project> = (1..=12)
            .map(|n| project(&format!("p{n}"), &format!("Project {n}"), Some(n), false))
            .collect();

        let rows = recent_rows(&projects, None);

        let listed: Vec<(&str, Option<&str>)> = rows
            .iter()
            .map(|r| (r.label.as_str(), r.accelerator.as_deref()))
            .collect();
        assert_eq!(
            listed,
            [
                ("Project 12", Some("Alt+1")),
                ("Project 11", Some("Alt+2")),
                ("Project 10", Some("Alt+3")),
                ("Project 9", Some("Alt+4")),
                ("Project 8", Some("Alt+5")),
                ("Project 7", Some("Alt+6")),
                ("Project 6", Some("Alt+7")),
                ("Project 5", Some("Alt+8")),
                ("Project 4", Some("Alt+9")),
                ("Project 3", Some("Alt+0")),
            ]
        );
    }

    #[test]
    fn the_current_projects_actions_come_apps_then_scripts_then_reveal_in_finder() {
        let config = config(json!({
            "apps": [
                { "label": "Open Docs", "url": "https://docs.example.com", "shortcut": "cmd+d" },
                { "label": "Open Admin", "url": "https://admin.example.com" },
            ],
            "scripts": [
                { "label": "Deploy", "command": "make deploy", "shortcut": "cmd+shift+k" },
            ],
        }));
        assert_eq!(
            current_rows("acme-website", &config),
            [
                row(
                    "Open Docs",
                    "project:launch:acme-website:apps:0",
                    Some("CmdOrCtrl+D")
                ),
                row("Open Admin", "project:launch:acme-website:apps:1", None),
                row(
                    "Deploy",
                    "project:launch:acme-website:scripts:0",
                    Some("CmdOrCtrl+Shift+K")
                ),
                row("Reveal in Finder", "project:reveal:acme-website", None),
            ]
        );
    }

    #[test]
    fn a_project_without_actions_offers_only_reveal_in_finder() {
        assert_eq!(
            current_rows("northwind", &config(json!({}))),
            [row("Reveal in Finder", "project:reveal:northwind", None)]
        );
    }

    #[test]
    fn an_action_shortcut_keeps_all_its_modifiers_in_the_accelerator() {
        let config = config(json!({
            "scripts": [
                { "label": "Build", "command": "make", "shortcut": "ctrl+alt+cmd+shift+b" },
            ],
        }));
        assert_eq!(
            current_rows("northwind", &config)[0].accelerator.as_deref(),
            Some("CmdOrCtrl+Ctrl+Alt+Shift+B")
        );
    }
}
