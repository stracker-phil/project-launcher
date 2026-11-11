//! Project edits: adding and removing projects, and the keys of a project's file Launcher changes.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};
use ts_rs::TS;

use super::{edit, invalid_field, non_blank};
use crate::data::ops::Op;
use crate::data::{Changed, Data, Edit};
use crate::error::AppResult;
use crate::projects::file::{self, FileObject};
use crate::projects::view::folder_name;
use crate::projects::{FileKey, FileRead, KnownProject, MetaKey};

/// A change to one `meta` key of a project's file; empty text, no note lines and `false` remove the key.
#[derive(Deserialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "field", content = "value", rename_all = "camelCase")]
pub enum MetaEdit {
    Icon(Option<String>),
    Color(Option<String>),
    /// A tag not in the tag list is added to it.
    Tag(Option<String>),
    Notes(Vec<String>),
    Starred(bool),
    Archived(bool),
}

impl MetaEdit {
    fn key(&self) -> MetaKey {
        match self {
            MetaEdit::Icon(_) => MetaKey::Icon,
            MetaEdit::Color(_) => MetaKey::Color,
            MetaEdit::Tag(_) => MetaKey::Tag,
            MetaEdit::Notes(_) => MetaKey::Notes,
            MetaEdit::Starred(_) => MetaKey::Starred,
            MetaEdit::Archived(_) => MetaKey::Archived,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            MetaEdit::Icon(_) => "Change Icon",
            MetaEdit::Color(_) => "Change Color",
            MetaEdit::Tag(_) => "Change Tag",
            MetaEdit::Notes(_) => "Edit Notes",
            MetaEdit::Starred(true) => "Star Project",
            MetaEdit::Starred(false) => "Unstar Project",
            MetaEdit::Archived(true) => "Archive Project",
            MetaEdit::Archived(false) => "Unarchive Project",
        }
    }

    /// `None` removes the key from the file.
    fn value(&self) -> Option<Value> {
        match self {
            MetaEdit::Icon(s) | MetaEdit::Color(s) | MetaEdit::Tag(s) => {
                s.as_deref().and_then(non_blank).map(Value::String)
            }
            MetaEdit::Notes(lines) => lines
                .iter()
                .any(|line| !line.trim().is_empty())
                .then(|| json!(lines)),
            MetaEdit::Starred(on) | MetaEdit::Archived(on) => on.then_some(Value::Bool(true)),
        }
    }

    /// The tag this edit sets, if any.
    fn new_tag(&self) -> Option<String> {
        match self {
            MetaEdit::Tag(tag) => tag.as_deref().and_then(non_blank),
            _ => None,
        }
    }
}

impl Data {
    /// Adds a folder (`~` for the home folder) and returns the new project's id. A folder without a project
    /// file gets one with its name (ADR 0038).
    pub fn add_project(&self, path: &str, now_ms: i64) -> AppResult<(String, Changed)> {
        let dir = self.folder(path)?;
        let id = new_id();
        let changed = self.change(|data| {
            if data.registry.read(|r| r.find_path(&dir).is_some()) {
                return Err(invalid_field("path", format!("{dir} is already in the list")));
            }
            let project = KnownProject {
                id: id.clone(),
                path: dir.clone(),
                added_at: now_ms,
                last_opened_at: None,
            };
            let index = data.registry.read(|r| r.projects.len());
            let add = Op::AddProject { project, index };
            if file::read(Path::new(&dir)) != FileRead::Missing {
                return edit("Add Project", add);
            }
            let mut named = Map::new();
            named.insert("name".into(), Value::String(folder_name(&dir)));
            let create = Op::ReplaceFile {
                dir: dir.clone(),
                before: None,
                after: Some(named),
            };
            edit("Add Project", Op::Batch(vec![create, add]))
        })?;
        Ok((id, changed))
    }

    /// Takes the project off the list; its folder and file stay as they are.
    pub fn remove_project(&self, id: &str) -> AppResult<Changed> {
        self.change(|data| {
            data.project_dir(id)?;
            edit("Remove Project", Op::RemoveProject { id: id.into() })
        })
    }

    /// An empty name removes it from the file, so the folder's name shows.
    pub fn set_project_name(&self, id: &str, name: &str) -> AppResult<Changed> {
        let value = non_blank(name).map(Value::String);
        self.change(|data| data.file_key_edit(id, "Rename Project", FileKey::Name, value))
    }

    pub fn set_project_meta(&self, id: &str, meta: MetaEdit) -> AppResult<Changed> {
        self.change(|data| {
            let key = FileKey::Meta(meta.key());
            let Some(Edit { label, op }) = data.file_key_edit(id, meta.label(), key, meta.value())?
            else {
                return Ok(None);
            };
            let mut tags = data.settings().tags;
            if let Some(tag) = meta.new_tag().filter(|tag| !tags.contains(tag)) {
                tags.push(tag);
                return edit(label, Op::Batch(vec![Op::SetTags(tags), op]));
            }
            edit(label, op)
        })
    }

    /// One key of the project's file, read fresh; nothing to do when it already holds `value`.
    fn file_key_edit(
        &self,
        id: &str,
        label: &'static str,
        key: FileKey,
        value: Option<Value>,
    ) -> AppResult<Option<Edit>> {
        let object: FileObject = file::read_for_change(Path::new(&self.project_dir(id)?))?;
        if key.get(&object) == value {
            return Ok(None);
        }
        let id = id.to_string();
        edit(label, Op::SetFileKey { id, key, value })
    }

    /// An existing folder as an absolute path without a trailing slash.
    fn folder(&self, raw: &str) -> AppResult<String> {
        let path = expand_home(raw.trim(), self.home.as_deref());
        if !path.is_absolute() {
            return Err(invalid_field("path", "Choose a folder by its full path"));
        }
        if !path.is_dir() {
            return Err(invalid_field("path", "No folder at this path"));
        }
        Ok(normalize(&path))
    }
}

/// `~` and `~/…` resolved against `home`; without a home they stay as typed.
fn expand_home(raw: &str, home: Option<&Path>) -> PathBuf {
    let Some(home) = home else {
        return PathBuf::from(raw);
    };
    if raw == "~" {
        return home.to_path_buf();
    }
    match raw.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(raw),
    }
}

/// The path without a trailing slash; the root stays `/`.
fn normalize(path: &Path) -> String {
    match path.to_string_lossy().trim_end_matches('/') {
        "" => "/".into(),
        trimmed => trimmed.into(),
    }
}

/// Twelve hex digits: short enough to type (`launcher open <id>`), unique enough for a project list.
fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}
