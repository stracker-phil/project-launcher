//! `projects.json`: the known project files, nothing else about a project (ADR 0038). Repaired per entry on
//! load (ADR 0013).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A project Launcher knows: its folder, when it was added and last opened (UTC milliseconds, ADR 0010).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnownProject {
    pub id: String,
    /// The absolute folder path.
    pub path: String,
    pub added_at: i64,
    pub last_opened_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Registry {
    pub projects: Vec<KnownProject>,
}

impl Registry {
    /// The entries of whatever `projects.json` holds: an entry without an id or an absolute path is dropped,
    /// as is a second entry with the same id or path; a bad time falls back (added: 0, last opened: none).
    pub fn repaired(raw: Value) -> Self {
        let mut ids = HashSet::new();
        let mut paths = HashSet::new();
        let projects = raw
            .get("projects")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let known = repaired_entry(entry);
                if known.is_none() {
                    log::warn!("Dropped an unusable entry from projects.json: {entry}");
                }
                known
            })
            .filter(|known| ids.insert(known.id.clone()) && paths.insert(known.path.clone()))
            .collect();
        Self { projects }
    }

    pub fn get(&self, id: &str) -> Option<&KnownProject> {
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn position(&self, id: &str) -> Option<usize> {
        self.projects.iter().position(|p| p.id == id)
    }

    pub fn find_path(&self, path: &str) -> Option<&KnownProject> {
        self.projects.iter().find(|p| p.path == path)
    }
}

fn repaired_entry(entry: &Value) -> Option<KnownProject> {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let id = text("id")?;
    let path = text("path")
        .map(|p| p.trim_end_matches('/'))
        .filter(|p| p.starts_with('/'))?;
    Some(KnownProject {
        id: id.to_string(),
        path: path.to_string(),
        added_at: entry.get("addedAt").and_then(Value::as_i64).unwrap_or(0),
        last_opened_at: entry.get("lastOpenedAt").and_then(Value::as_i64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn known(id: &str, path: &str) -> KnownProject {
        KnownProject {
            id: id.into(),
            path: path.into(),
            added_at: 0,
            last_opened_at: None,
        }
    }

    fn repaired(entries: Value) -> Vec<KnownProject> {
        Registry::repaired(json!({ "projects": entries })).projects
    }

    #[test]
    fn a_valid_entry_is_kept_whole() {
        let projects = repaired(json!([
            {"id": "northwind", "path": "/work/northwind", "addedAt": 1000, "lastOpenedAt": 2000}
        ]));

        assert_eq!(
            projects,
            [KnownProject {
                id: "northwind".into(),
                path: "/work/northwind".into(),
                added_at: 1000,
                last_opened_at: Some(2000),
            }]
        );
    }

    #[test]
    fn anything_but_an_object_with_a_projects_list_is_empty() {
        for raw in [
            json!(null),
            json!([]),
            json!({}),
            json!({"projects": "x"}),
            json!({"projects": {}}),
        ] {
            assert_eq!(
                Registry::repaired(raw.clone()),
                Registry::default(),
                "{raw}"
            );
        }
    }

    #[test]
    fn entries_without_a_usable_id_are_dropped() {
        let projects = repaired(json!([
            {"path": "/work/a"},
            {"id": "", "path": "/work/b"},
            {"id": "   ", "path": "/work/c"},
            {"id": 7, "path": "/work/d"},
            "not an entry",
            {"id": "ok", "path": "/work/e"},
        ]));

        assert_eq!(projects, [known("ok", "/work/e")]);
    }

    #[test]
    fn entries_without_an_absolute_path_are_dropped() {
        let projects = repaired(json!([
            {"id": "a"},
            {"id": "b", "path": ""},
            {"id": "c", "path": "work/relative"},
            {"id": "d", "path": "~/work/home"},
            {"id": "e", "path": "/"},
            {"id": "g", "path": 5},
            {"id": "ok", "path": "/work/ok"},
        ]));

        assert_eq!(projects, [known("ok", "/work/ok")]);
    }

    #[test]
    fn a_path_of_only_slashes_is_dropped() {
        let projects = repaired(json!([
            {"id": "a", "path": "/"},
            {"id": "b", "path": "//"},
            {"id": "c", "path": "///"},
            {"id": "ok", "path": "/a//"},
        ]));

        assert_eq!(projects, [known("ok", "/a")]);
    }

    #[test]
    fn trailing_slashes_are_trimmed_and_ids_and_paths_are_trimmed() {
        let projects = repaired(json!([
            {"id": " a ", "path": " /work/a/ "},
            {"id": "b", "path": "/work/b///"},
        ]));

        assert_eq!(projects, [known("a", "/work/a"), known("b", "/work/b")]);
    }

    #[test]
    fn a_second_entry_with_the_same_id_is_dropped() {
        let projects = repaired(json!([
            {"id": "a", "path": "/work/a", "addedAt": 1},
            {"id": "a", "path": "/work/other", "addedAt": 2},
        ]));

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path, "/work/a");
        assert_eq!(projects[0].added_at, 1);
    }

    #[test]
    fn a_second_entry_with_the_same_path_is_dropped_even_with_a_trailing_slash() {
        let projects = repaired(json!([
            {"id": "a", "path": "/work/a"},
            {"id": "b", "path": "/work/a/"},
            {"id": "c", "path": "/work/c"},
        ]));

        assert_eq!(projects, [known("a", "/work/a"), known("c", "/work/c")]);
    }

    #[test]
    fn bad_times_fall_back_to_zero_and_none() {
        let projects = repaired(json!([
            {"id": "a", "path": "/work/a", "addedAt": "yesterday", "lastOpenedAt": "today"},
            {"id": "b", "path": "/work/b", "addedAt": 1.5, "lastOpenedAt": null},
            {"id": "c", "path": "/work/c"},
        ]));

        assert_eq!(
            projects,
            [
                known("a", "/work/a"),
                known("b", "/work/b"),
                known("c", "/work/c")
            ]
        );
    }

    #[test]
    fn lookups_find_entries_by_id_position_and_path() {
        let registry = Registry {
            projects: vec![known("a", "/work/a"), known("b", "/work/b")],
        };

        assert_eq!(registry.get("b"), Some(&known("b", "/work/b")));
        assert_eq!(registry.get("zzz"), None);
        assert_eq!(registry.position("b"), Some(1));
        assert_eq!(registry.position("zzz"), None);
        assert_eq!(registry.find_path("/work/a"), Some(&known("a", "/work/a")));
        assert_eq!(registry.find_path("/work/zzz"), None);
    }

    #[test]
    fn the_registry_serializes_with_camel_case_entry_fields() {
        let registry = Registry {
            projects: vec![KnownProject {
                id: "a".into(),
                path: "/work/a".into(),
                added_at: 5,
                last_opened_at: Some(9),
            }],
        };

        assert_eq!(
            serde_json::to_value(&registry).unwrap(),
            json!({"projects": [
                {"id": "a", "path": "/work/a", "addedAt": 5, "lastOpenedAt": 9}
            ]})
        );
    }
}
