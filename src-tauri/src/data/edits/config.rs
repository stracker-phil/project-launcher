//! Launch config edits: actions, built-ins, the lock and the dev mode. Each reads the project's file fresh,
//! and changes only the keys it needs, as one undoable step. A locked project refuses every edit of its
//! details except unlocking, starring and archiving.

use std::path::Path;

use serde_json::Value;

use super::edit;
use crate::data::ops::Op;
use crate::data::{Changed, Data, Edit};
use crate::error::{AppError, AppResult};
use crate::projects::FileKey;
use crate::projects::config::edit::Lists;
use crate::projects::config::{ActionDraft, ActionSource, Builtin, DevMode};
use crate::projects::file::{self, FileObject};

impl Data {
    /// Creates (`source: None`) or updates an action. `preset` names the built-in a draft overrides.
    pub fn save_action(
        &self,
        id: &str,
        source: Option<ActionSource>,
        draft: &ActionDraft,
        preset: Option<Builtin>,
    ) -> AppResult<Changed> {
        let label = if source.is_some() {
            "Edit Action"
        } else {
            "New Action"
        };
        self.lists_edit(id, label, |lists| lists.save(source, draft, preset))
    }

    pub fn remove_action(&self, id: &str, source: ActionSource) -> AppResult<Changed> {
        self.lists_edit(id, "Remove Action", |lists| lists.remove(source))
    }

    pub fn move_action(&self, id: &str, source: ActionSource, to: usize) -> AppResult<Changed> {
        self.lists_edit(id, "Move Action", |lists| lists.reorder(source, to))
    }

    pub fn set_builtin_enabled(
        &self,
        id: &str,
        preset: Builtin,
        enabled: bool,
    ) -> AppResult<Changed> {
        let label = if enabled {
            "Add Built-in App"
        } else {
            "Remove Built-in App"
        };
        self.lists_edit(id, label, |lists| {
            lists.set_builtin(preset, enabled);
            Ok(())
        })
    }

    pub fn set_project_locked(&self, id: &str, locked: bool) -> AppResult<Changed> {
        let label = if locked {
            "Lock Project Details"
        } else {
            "Unlock Project Details"
        };
        self.change(|data| {
            let object = data.project_file(id)?;
            key_edit(
                id,
                label,
                &object,
                FileKey::Locked,
                locked.then_some(Value::Bool(true)),
            )
        })
    }

    pub fn set_dev_mode(&self, id: &str, mode: DevMode) -> AppResult<Changed> {
        self.change(|data| {
            let object = data.unlocked_project_file(id)?;
            key_edit(
                id,
                "Change Dev Environment",
                &object,
                FileKey::Dev,
                mode.value(),
            )
        })
    }

    /// The project's file read fresh, for a change.
    pub(super) fn project_file(&self, id: &str) -> AppResult<FileObject> {
        file::read_for_change(Path::new(&self.project_dir(id)?))
    }

    /// The project's file, refused while its details are locked.
    pub(super) fn unlocked_project_file(&self, id: &str) -> AppResult<FileObject> {
        let object = self.project_file(id)?;
        if object.get("locked") == Some(&Value::Bool(true)) {
            return Err(AppError::InvalidInput(
                "The project's details are locked; unlock them first".into(),
            ));
        }
        Ok(object)
    }

    fn lists_edit(
        &self,
        id: &str,
        label: &'static str,
        change: impl FnOnce(&mut Lists) -> AppResult<()>,
    ) -> AppResult<Changed> {
        self.change(|data| {
            let mut lists = Lists::of(&data.unlocked_project_file(id)?);
            change(&mut lists)?;
            let ops: Vec<Op> = lists
                .changes()
                .into_iter()
                .map(|(key, value)| set_key(id, key, Some(value)))
                .collect();
            if ops.is_empty() {
                return Ok(None);
            }
            edit(label, Op::Batch(ops))
        })
    }
}

/// One key of `object`; nothing to do when it already holds `value`.
pub(super) fn key_edit(
    id: &str,
    label: &'static str,
    object: &FileObject,
    key: FileKey,
    value: Option<Value>,
) -> AppResult<Option<Edit>> {
    if key.get(object) == value {
        return Ok(None);
    }
    edit(label, set_key(id, key, value))
}

pub(super) fn set_key(id: &str, key: FileKey, value: Option<Value>) -> Op {
    Op::SetFileKey {
        id: id.to_string(),
        key,
        value,
    }
}
