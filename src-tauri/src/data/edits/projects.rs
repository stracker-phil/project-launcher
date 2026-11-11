//! Project edits: adding and removing projects, and the keys of a project's file Launcher changes.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use ts_rs::TS;

use super::config::{key_edit, set_key};
use super::{edit, invalid_field, non_blank};
use crate::data::ops::Op;
use crate::data::{Changed, Data, Edit};
use crate::error::AppResult;
use crate::projects::file;
use crate::projects::view::folder_name;
use crate::projects::{FileKey, FileRead, KnownProject, MetaKey};

/// What adding folders did: the new projects' ids, and the ids of folders that were listed already.
#[derive(Serialize, TS, Debug, Clone, PartialEq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct AddedProjects {
    pub added: Vec<String>,
    pub existing: Vec<String>,
}

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
    /// The website: the browser built-in opens it, `${url}` stands for it.
    Url(Option<String>),
    RepoUrl(Option<String>),
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
            MetaEdit::Url(_) => MetaKey::Url,
            MetaEdit::RepoUrl(_) => MetaKey::RepoUrl,
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
            MetaEdit::Url(_) => "Change Website",
            MetaEdit::RepoUrl(_) => "Change Repository",
        }
    }

    /// `None` removes the key from the file.
    fn value(&self) -> Option<Value> {
        match self {
            MetaEdit::Icon(s)
            | MetaEdit::Color(s)
            | MetaEdit::Tag(s)
            | MetaEdit::Url(s)
            | MetaEdit::RepoUrl(s) => s.as_deref().and_then(non_blank).map(Value::String),
            MetaEdit::Notes(lines) => lines
                .iter()
                .any(|line| !line.trim().is_empty())
                .then(|| json!(lines)),
            MetaEdit::Starred(on) | MetaEdit::Archived(on) => on.then_some(Value::Bool(true)),
        }
    }

    /// Starring and archiving work on a locked project; the other details don't.
    fn works_while_locked(&self) -> bool {
        matches!(self, MetaEdit::Starred(_) | MetaEdit::Archived(_))
    }

    /// A starred project isn't archived, and the other way round.
    fn excludes(&self) -> Option<MetaKey> {
        match self {
            MetaEdit::Starred(true) => Some(MetaKey::Archived),
            MetaEdit::Archived(true) => Some(MetaKey::Starred),
            _ => None,
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
        if self.registry.read(|r| r.find_path(&dir).is_some()) {
            return Err(invalid_field(
                "path",
                format!("{dir} is already in the list"),
            ));
        }
        let (added, changed) = self.add_projects(&[dir], now_ms)?;
        Ok((added.added.into_iter().next().unwrap_or_default(), changed))
    }

    /// Adds the folders that aren't listed yet, in one step; listed ones come back as `existing`.
    pub fn add_projects(
        &self,
        paths: &[String],
        now_ms: i64,
    ) -> AppResult<(AddedProjects, Changed)> {
        let dirs = paths
            .iter()
            .map(|path| self.folder(path))
            .collect::<AppResult<Vec<_>>>()?;
        let mut result = AddedProjects::default();
        let changed = self.change(|data| {
            let mut ops = Vec::new();
            let mut seen = HashSet::new();
            let mut index = data.registry.read(|r| r.projects.len());
            for dir in dirs {
                if let Some(known) = data
                    .registry
                    .read(|r| r.find_path(&dir).map(|p| p.id.clone()))
                {
                    result.existing.push(known);
                    continue;
                }
                if !seen.insert(dir.clone()) {
                    continue;
                }
                let id = new_id();
                if file::read(Path::new(&dir)) == FileRead::Missing {
                    let mut named = Map::new();
                    named.insert("name".into(), Value::String(folder_name(&dir)));
                    ops.push(Op::ReplaceFile {
                        dir: dir.clone(),
                        before: None,
                        after: Some(named),
                    });
                }
                let project = KnownProject {
                    id: id.clone(),
                    path: dir,
                    added_at: now_ms,
                    last_opened_at: None,
                };
                ops.push(Op::AddProject { project, index });
                index += 1;
                result.added.push(id);
            }
            let label = match result.added.len() {
                0 => return Ok(None),
                1 => "Add Project",
                _ => "Add Projects",
            };
            edit(label, Op::Batch(ops))
        })?;
        Ok((result, changed))
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
        self.change(|data| {
            let object = data.unlocked_project_file(id)?;
            key_edit(id, "Rename Project", &object, FileKey::Name, value)
        })
    }

    pub fn set_project_meta(&self, id: &str, meta: MetaEdit) -> AppResult<Changed> {
        self.change(|data| {
            let object = if meta.works_while_locked() {
                data.project_file(id)?
            } else {
                data.unlocked_project_file(id)?
            };
            let key = FileKey::Meta(meta.key());
            let Some(Edit { label, op }) = key_edit(id, meta.label(), &object, key, meta.value())?
            else {
                return Ok(None);
            };
            let mut ops = vec![op];
            if let Some(other) = meta.excludes()
                && FileKey::Meta(other).get(&object).is_some()
            {
                ops.push(set_key(id, FileKey::Meta(other), None));
            }
            let mut tags = data.settings().tags;
            if let Some(tag) = meta.new_tag().filter(|tag| !tags.contains(tag)) {
                tags.push(tag);
                ops.insert(0, Op::SetTags(tags));
            }
            let op = if ops.len() == 1 {
                ops.remove(0)
            } else {
                Op::Batch(ops)
            };
            edit(label, op)
        })
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
