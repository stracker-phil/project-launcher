//! Prints the running app's replies for people; `--json` prints them as they came.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::automation::import::Plan;
use crate::automation::{Description, ListedProject};
use crate::ipc::IpcMessage;

/// The reply to `launcher status`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusReport {
    pub version: String,
    pub window_visible: bool,
}

/// A command of `launcher commands`.
#[derive(Deserialize)]
struct CommandRow {
    id: String,
    title: String,
}

/// The text for `result`, the reply to `message`; `None` when the subcommand prints nothing.
pub fn render(message: &IpcMessage, result: &Value) -> Result<Option<String>, String> {
    Ok(match message {
        IpcMessage::Status => Some(status_text(&read::<StatusReport>(result)?)),
        IpcMessage::List => Some(list_text(&read::<Vec<ListedProject>>(result)?)),
        IpcMessage::Describe { .. } => Some(describe_text(&read::<Description>(result)?)),
        IpcMessage::Commands => Some(commands_text(&read::<Vec<CommandRow>>(result)?)),
        IpcMessage::Run { .. } => script_output(result),
        IpcMessage::Db { .. } if result.as_str() == Some("starting") => {
            Some("Starting the dev server first; DataBee opens once it runs.".into())
        }
        IpcMessage::Import { yes, .. } => Some(import_text(&read::<Plan>(result)?, *yes)),
        _ => None,
    })
}

/// Columns padded to the widest cell.
fn table(rows: &[Vec<String>]) -> String {
    let widths: Vec<usize> = (0..rows.first().map_or(0, Vec::len))
        .map(|column| {
            rows.iter()
                .map(|row| row[column].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    rows.iter()
        .map(|row| {
            row.iter()
                .zip(&widths)
                .map(|(cell, width)| format!("{cell:<width$}"))
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The home folder as `~`.
fn tilde(path: &str) -> String {
    let home = std::env::home_dir();
    match home
        .as_deref()
        .and_then(|home| Path::new(path).strip_prefix(home).ok())
    {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.to_string(),
    }
}

fn script_output(result: &Value) -> Option<String> {
    let output = result.get("output")?.as_str()?;
    (!output.is_empty()).then(|| output.to_string())
}

fn list_text(projects: &[ListedProject]) -> String {
    if projects.is_empty() {
        return "No projects yet.".into();
    }
    let mut rows = vec![vec!["NAME".to_string(), "TAG".into(), "PATH".into()]];
    rows.extend(projects.iter().map(|p| {
        let name = if p.archived {
            format!("{} (archived)", p.name)
        } else {
            p.name.clone()
        };
        vec![name, p.tag.clone().unwrap_or_default(), tilde(&p.path)]
    }));
    table(&rows)
}

fn describe_text(d: &Description) -> String {
    let p = &d.project;
    let mut lines = vec![p.name.clone(), tilde(&p.path)];
    if let Some(tag) = &p.tag {
        lines.push(format!("Tag: {tag}"));
    }
    for (label, value) in [("Website", &d.url), ("Repository", &d.repo_url)] {
        if let Some(value) = value {
            lines.push(format!("{label}: {value}"));
        }
    }
    if let Some(error) = &d.file_error {
        lines.push(format!("Problem: {error}"));
    }
    for (title, entries) in [("Apps", &d.apps), ("Scripts", &d.scripts)] {
        if entries.is_empty() {
            continue;
        }
        lines.push(String::new());
        lines.push(format!("{title}:"));
        let rows: Vec<Vec<String>> = entries
            .iter()
            .map(|(label, what)| vec![format!("  {label}"), what.clone()])
            .collect();
        lines.push(table(&rows));
    }
    lines.join("\n")
}

fn commands_text(commands: &[CommandRow]) -> String {
    let rows: Vec<Vec<String>> = commands
        .iter()
        .map(|c| vec![c.id.clone(), c.title.clone()])
        .collect();
    table(&rows)
}

fn import_text(plan: &Plan, applied: bool) -> String {
    let mut lines = Vec::new();
    let mut section = |title: &str, items: &[String]| {
        if !items.is_empty() {
            lines.push(format!("{title}:"));
            lines.extend(items.iter().map(|item| format!("  {item}")));
        }
    };
    section(if applied { "Added" } else { "Adds" }, &plan.added);
    section("Already in the list", &plan.known);
    section(if applied { "Wrote" } else { "Writes" }, &plan.files);
    section("Settings", &plan.settings);
    section("Tags", &plan.tags);
    let skipped: Vec<String> = plan
        .skipped
        .iter()
        .map(|s| format!("{} ({})", s.path, s.reason))
        .collect();
    section("Skipped", &skipped);
    if lines.is_empty() {
        return "Nothing to import.".into();
    }
    lines.push(String::new());
    lines.push(if applied {
        "Imported. Edit → Undo in Launcher takes it back.".into()
    } else {
        "Nothing changed yet. Run again with --yes to import.".into()
    });
    lines.join("\n")
}

fn read<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    T::deserialize(value).map_err(|e| format!("The running Launcher sent an unexpected reply: {e}"))
}

fn status_text(report: &StatusReport) -> String {
    let window = if report.window_visible {
        "visible"
    } else {
        "hidden"
    };
    format!("Launcher {}\nwindow: {window}", report.version)
}
