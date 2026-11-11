//! A project as the page shows it: its registry entry plus the name and `meta` from its file.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::file::{FileObject, FileRead};
use super::registry::KnownProject;
use crate::status::tilde;

/// What a project without its own icon or colour shows.
pub const DEFAULT_ICON: &str = "Folder";
pub const DEFAULT_COLOR: &str = "Blue";

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    /// The folder, starting with `~` for the home folder.
    pub path: String,
    /// The file's `name`, else the folder's name.
    pub name: String,
    pub meta: ProjectMeta,
    #[ts(type = "number")]
    pub added_at: i64,
    #[ts(type = "number | null")]
    pub last_opened_at: Option<i64>,
    /// False when the project's file is missing or broken; the name is then the folder's.
    pub available: bool,
    /// The details are read-only in the app (`"locked": true`).
    pub locked: bool,
    /// Why the file can't be used.
    pub file_error: Option<String>,
}

/// The file's `meta`, cleaned up: blank strings count as unset, notes are lines.
#[derive(Serialize, TS, Debug, Clone, PartialEq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub icon: String,
    pub color: String,
    pub tag: Option<String>,
    /// Empty when no line has text.
    pub notes: Vec<String>,
    pub starred: bool,
    pub archived: bool,
    pub url: Option<String>,
    pub repo_url: Option<String>,
    /// The editor this project opens in, instead of the default editor.
    pub editor: Option<String>,
}

pub fn folder_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

pub fn build(known: &KnownProject, file: &FileRead, home: Option<&Path>) -> Project {
    let (object, file_error) = match file {
        FileRead::Found(object) => (Some(object), None),
        FileRead::Missing => (None, Some(format!("{} is missing", super::FILE_NAME))),
        FileRead::Broken(problem) => (None, Some(problem.clone())),
    };
    let name = object
        .and_then(|o| text(o.get("name")))
        .unwrap_or_else(|| folder_name(&known.path));
    Project {
        id: known.id.clone(),
        path: tilde(Path::new(&known.path), home),
        name,
        meta: object.map(meta_of).unwrap_or_else(default_meta),
        added_at: known.added_at,
        last_opened_at: known.last_opened_at,
        available: object.is_some(),
        locked: object.is_some_and(|o| o.get("locked") == Some(&Value::Bool(true))),
        file_error,
    }
}

fn default_meta() -> ProjectMeta {
    ProjectMeta {
        icon: DEFAULT_ICON.into(),
        color: DEFAULT_COLOR.into(),
        ..ProjectMeta::default()
    }
}

/// The trimmed string, when there is one with text.
pub fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Notes as lines: an array keeps its strings, a string is split at line breaks; no text means none.
pub fn notes(value: Option<&Value>) -> Vec<String> {
    let lines: Vec<String> = match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(s)) => s.lines().map(str::to_string).collect(),
        _ => Vec::new(),
    };
    if lines.iter().any(|line| !line.trim().is_empty()) {
        lines
    } else {
        Vec::new()
    }
}

fn meta_of(object: &FileObject) -> ProjectMeta {
    let meta = object.get("meta");
    let field = |key: &str| meta.and_then(|m| m.get(key));
    ProjectMeta {
        icon: text(field("icon")).unwrap_or_else(|| DEFAULT_ICON.into()),
        color: text(field("color")).unwrap_or_else(|| DEFAULT_COLOR.into()),
        tag: text(field("tag")),
        notes: notes(field("notes")),
        starred: field("starred") == Some(&Value::Bool(true)),
        archived: field("archived") == Some(&Value::Bool(true)),
        url: text(field("url")),
        repo_url: text(field("repoUrl")),
        editor: text(field("editor")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn known(path: &str) -> KnownProject {
        KnownProject {
            id: "northwind".into(),
            path: path.into(),
            added_at: 1000,
            last_opened_at: Some(2000),
        }
    }

    fn found(value: Value) -> FileRead {
        let Value::Object(object) = value else {
            panic!("not an object");
        };
        FileRead::Found(object)
    }

    fn meta_of_file(meta: Value) -> ProjectMeta {
        build(
            &known("/work/northwind"),
            &found(json!({ "meta": meta })),
            None,
        )
        .meta
    }

    #[test]
    fn a_found_file_gives_name_meta_and_the_registry_times() {
        let file = found(json!({
            "name": "  Northwind Shop ",
            "meta": {
                "icon": "Bolt",
                "color": "Red",
                "tag": "client",
                "notes": ["call back"],
                "starred": true,
                "archived": true,
                "url": "https://example.com",
                "repoUrl": "https://example.com/repo",
                "editor": "Zed",
            },
        }));

        assert_eq!(
            build(&known("/work/northwind"), &file, None),
            Project {
                id: "northwind".into(),
                path: "/work/northwind".into(),
                name: "Northwind Shop".into(),
                meta: ProjectMeta {
                    icon: "Bolt".into(),
                    color: "Red".into(),
                    tag: Some("client".into()),
                    notes: vec!["call back".into()],
                    starred: true,
                    archived: true,
                    url: Some("https://example.com".into()),
                    repo_url: Some("https://example.com/repo".into()),
                    editor: Some("Zed".into()),
                },
                added_at: 1000,
                last_opened_at: Some(2000),
                available: true,
                locked: false,
                file_error: None,
            }
        );
    }

    #[test]
    fn locked_is_set_only_by_a_file_with_locked_true() {
        for (file, expected) in [
            (found(json!({"locked": true})), true),
            (found(json!({"locked": false})), false),
            (found(json!({"locked": "true"})), false),
            (found(json!({"locked": 1})), false),
            (found(json!({})), false),
            (FileRead::Missing, false),
            (FileRead::Broken("broken".into()), false),
        ] {
            let project = build(&known("/work/northwind"), &file, None);
            assert_eq!(project.locked, expected, "{file:?}");
        }
    }

    #[test]
    fn the_name_falls_back_to_the_folder_name() {
        for file in [
            found(json!({})),
            found(json!({"name": ""})),
            found(json!({"name": "   "})),
            found(json!({"name": 5})),
        ] {
            let project = build(&known("/work/northwind"), &file, None);
            assert_eq!(project.name, "northwind", "{file:?}");
        }
    }

    #[test]
    fn a_file_without_meta_shows_the_default_icon_and_colour() {
        let meta = build(
            &known("/work/northwind"),
            &found(json!({"name": "A"})),
            None,
        )
        .meta;

        assert_eq!(
            meta,
            ProjectMeta {
                icon: "Folder".into(),
                color: "Blue".into(),
                ..ProjectMeta::default()
            }
        );
    }

    #[test]
    fn blank_or_non_string_icon_and_colour_fall_back_to_the_defaults() {
        for value in [json!(""), json!("  "), json!(3), json!(null)] {
            let meta = meta_of_file(json!({"icon": value.clone(), "color": value.clone()}));
            assert_eq!(
                (meta.icon.as_str(), meta.color.as_str()),
                ("Folder", "Blue"),
                "{value}"
            );
        }
    }

    #[test]
    fn text_meta_is_trimmed_and_blank_counts_as_unset() {
        let meta = meta_of_file(json!({
            "tag": " client ",
            "url": "   ",
            "repoUrl": "",
            "editor": 4,
        }));

        assert_eq!(meta.tag.as_deref(), Some("client"));
        assert_eq!(meta.url, None);
        assert_eq!(meta.repo_url, None);
        assert_eq!(meta.editor, None);
    }

    #[test]
    fn notes_as_a_string_are_split_at_line_breaks() {
        let meta = meta_of_file(json!({"notes": "first\nsecond\n\nfourth"}));
        assert_eq!(meta.notes, ["first", "second", "", "fourth"]);
    }

    #[test]
    fn notes_as_a_string_with_windows_line_breaks_have_no_carriage_returns() {
        let meta = meta_of_file(json!({"notes": "one\r\ntwo\r\nthree"}));
        assert_eq!(meta.notes, ["one", "two", "three"]);
    }

    #[test]
    fn notes_as_an_array_keep_their_strings_only() {
        let meta = meta_of_file(json!({"notes": ["first", 4, null, "second"]}));
        assert_eq!(meta.notes, ["first", "second"]);
    }

    #[test]
    fn notes_without_text_are_empty() {
        for notes in [
            json!(""),
            json!("  \n \n"),
            json!([]),
            json!(["", "   "]),
            json!([1, 2]),
            json!(5),
            json!(null),
        ] {
            let meta = meta_of_file(json!({ "notes": notes.clone() }));
            assert!(meta.notes.is_empty(), "{notes}");
        }
    }

    #[test]
    fn starred_and_archived_are_set_only_by_true() {
        for (value, expected) in [
            (json!(true), true),
            (json!(false), false),
            (json!("true"), false),
            (json!(1), false),
            (json!(null), false),
        ] {
            let meta = meta_of_file(json!({"starred": value.clone(), "archived": value.clone()}));
            assert_eq!(
                (meta.starred, meta.archived),
                (expected, expected),
                "{value}"
            );
        }
    }

    #[test]
    fn a_missing_file_makes_the_project_unavailable_with_the_folder_name() {
        let project = build(&known("/work/northwind"), &FileRead::Missing, None);

        assert!(!project.available);
        assert_eq!(
            project.file_error.as_deref(),
            Some(".project-launcher.json is missing")
        );
        assert_eq!(project.name, "northwind");
        assert_eq!(project.meta.icon, "Folder");
        assert_eq!(project.meta.color, "Blue");
    }

    #[test]
    fn a_broken_file_makes_the_project_unavailable_with_its_problem() {
        let file = FileRead::Broken(".project-launcher.json must hold a JSON object".into());

        let project = build(&known("/work/northwind"), &file, None);

        assert!(!project.available);
        assert_eq!(
            project.file_error.as_deref(),
            Some(".project-launcher.json must hold a JSON object")
        );
        assert_eq!(project.name, "northwind");
        assert_eq!(
            project.meta,
            build(&known("/work/northwind"), &FileRead::Missing, None).meta
        );
    }

    #[test]
    fn a_found_file_is_available_without_an_error() {
        let project = build(&known("/work/northwind"), &found(json!({})), None);

        assert!(project.available);
        assert_eq!(project.file_error, None);
    }

    #[test]
    fn the_home_folder_is_shown_as_a_tilde() {
        let home = Path::new("/Users/demo");

        for (path, expected) in [
            ("/Users/demo/code/northwind", "~/code/northwind"),
            ("/Users/demo", "~"),
            ("/work/northwind", "/work/northwind"),
            ("/Users/demonstration/x", "/Users/demonstration/x"),
        ] {
            let project = build(&known(path), &found(json!({})), Some(home));
            assert_eq!(project.path, expected, "{path}");
        }
    }

    #[test]
    fn without_a_home_folder_the_path_is_shown_whole() {
        let project = build(
            &known("/Users/demo/code/northwind"),
            &found(json!({})),
            None,
        );
        assert_eq!(project.path, "/Users/demo/code/northwind");
    }

    #[test]
    fn a_project_serializes_with_camel_case_fields() {
        let project = build(
            &known("/work/northwind"),
            &found(json!({"name": "A", "meta": {"repoUrl": "https://example.com/r"}})),
            None,
        );

        assert_eq!(
            serde_json::to_value(&project).unwrap(),
            json!({
                "id": "northwind",
                "path": "/work/northwind",
                "name": "A",
                "meta": {
                    "icon": "Folder",
                    "color": "Blue",
                    "tag": null,
                    "notes": [],
                    "starred": false,
                    "archived": false,
                    "url": null,
                    "repoUrl": "https://example.com/r",
                    "editor": null,
                },
                "addedAt": 1000,
                "lastOpenedAt": 2000,
                "available": true,
                "locked": false,
                "fileError": null,
            })
        );
    }

    #[test]
    fn folder_name_is_the_last_path_component() {
        assert_eq!(folder_name("/work/northwind"), "northwind");
        assert_eq!(folder_name("/"), "/");
    }
}
