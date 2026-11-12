//! Every write is an `Op`. Applying one returns its inverse, computed from what it replaced, so the undo stack
//! can take any change back (ADR 0015).

use std::path::Path;

use serde_json::Value;

use super::{Changed, Data};
use crate::error::{AppError, AppResult};
use crate::projects::file::{self, FileObject};
use crate::projects::{FileKey, FileRead, KnownProject, Registry};
use crate::store::{Accent, AppSetting, Appearance, DockerManager};

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    SetAppearance(Appearance),
    SetAccent(Accent),
    SetApp(AppSetting, String),
    SetDockerManager(DockerManager),
    /// Overrides the command's key; `None` leaves it without one.
    SetShortcut {
        command: String,
        key: Option<String>,
    },
    /// Drops the override, so the command's default key applies.
    ResetShortcut {
        command: String,
    },
    SetTags(Vec<String>),
    /// Inserts the entry at `index` (or last); refused when its id or folder is already known.
    AddProject {
        project: KnownProject,
        index: usize,
    },
    RemoveProject {
        id: String,
    },
    /// Sets (`Some`) or removes (`None`) one key of a known project's file, read fresh from disk.
    SetFileKey {
        id: String,
        key: FileKey,
        value: Option<Value>,
    },
    /// Replaces a project file's whole content (`None`: no file); refused when the file isn't `before`
    /// anymore, so a hand edit is never lost.
    ReplaceFile {
        dir: String,
        before: Option<FileObject>,
        after: Option<FileObject>,
    },
    /// Applied in order and taken back as one; a failure takes back what was applied.
    Batch(Vec<Op>),
}

/// An applied op: how to take it back and what it changed.
pub struct Applied {
    pub inverse: Op,
    pub changed: Changed,
}

pub fn apply(data: &Data, op: &Op) -> AppResult<Applied> {
    match op {
        Op::SetAppearance(appearance) => {
            let before = data
                .settings
                .update(|s| Ok(std::mem::replace(&mut s.appearance, *appearance)))?;
            Ok(settings_change(Op::SetAppearance(before)))
        }
        Op::SetAccent(accent) => {
            let before = data
                .settings
                .update(|s| Ok(std::mem::replace(&mut s.accent, *accent)))?;
            Ok(settings_change(Op::SetAccent(before)))
        }
        Op::SetApp(setting, value) => {
            let before = data
                .settings
                .update(|s| Ok(std::mem::replace(setting.of(s), value.clone())))?;
            Ok(settings_change(Op::SetApp(*setting, before)))
        }
        Op::SetDockerManager(manager) => {
            let before = data
                .settings
                .update(|s| Ok(std::mem::replace(&mut s.docker_manager, *manager)))?;
            Ok(settings_change(Op::SetDockerManager(before)))
        }
        Op::SetShortcut { command, key } => {
            let before = data
                .settings
                .update(|s| Ok(s.shortcuts.insert(command.clone(), key.clone())))?;
            Ok(settings_change(shortcut_restore(command, before)))
        }
        Op::ResetShortcut { command } => {
            let before = data.settings.update(|s| Ok(s.shortcuts.remove(command)))?;
            Ok(settings_change(shortcut_restore(command, before)))
        }
        Op::SetTags(tags) => {
            let before = data
                .settings
                .update(|s| Ok(std::mem::replace(&mut s.tags, tags.clone())))?;
            Ok(settings_change(Op::SetTags(before)))
        }
        Op::AddProject { project, index } => add_project(data, project, *index),
        Op::RemoveProject { id } => {
            let (project, index) = data.registry.update(|r| {
                let index = r.position(id).ok_or_else(|| project_not_found(id))?;
                Ok((r.projects.remove(index), index))
            })?;
            Ok(projects_change(Op::AddProject { project, index }))
        }
        Op::SetFileKey { id, key, value } => set_file_key(data, id, *key, value.clone()),
        Op::ReplaceFile { dir, before, after } => {
            replace_file(Path::new(dir), before.as_ref(), after.as_ref())?;
            data.cache_file(
                dir,
                after.clone().map_or(FileRead::Missing, FileRead::Found),
            );
            Ok(projects_change(Op::ReplaceFile {
                dir: dir.clone(),
                before: after.clone(),
                after: before.clone(),
            }))
        }
        Op::Batch(ops) => apply_batch(data, ops),
    }
}

/// The op that puts a command's shortcut override back to `before` (`None`: there was none).
fn shortcut_restore(command: &str, before: Option<Option<String>>) -> Op {
    let command = command.to_string();
    match before {
        Some(key) => Op::SetShortcut { command, key },
        None => Op::ResetShortcut { command },
    }
}

fn add_project(data: &Data, project: &KnownProject, index: usize) -> AppResult<Applied> {
    data.registry.update(|r| {
        ensure_not_listed(r, project)?;
        r.projects
            .insert(index.min(r.projects.len()), project.clone());
        Ok(())
    })?;
    Ok(projects_change(Op::RemoveProject {
        id: project.id.clone(),
    }))
}

fn ensure_not_listed(registry: &Registry, project: &KnownProject) -> AppResult<()> {
    if registry.get(&project.id).is_some() || registry.find_path(&project.path).is_some() {
        return Err(AppError::InvalidInput(format!(
            "{} is already in the list",
            project.path
        )));
    }
    Ok(())
}

fn set_file_key(data: &Data, id: &str, key: FileKey, value: Option<Value>) -> AppResult<Applied> {
    let dir = data.project_dir(id)?;
    let mut object = file::read_for_change(Path::new(&dir))?;
    let before = key.get(&object);
    key.set(&mut object, value);
    file::write(Path::new(&dir), &object)?;
    data.cache_file(&dir, FileRead::Found(object));
    Ok(projects_change(Op::SetFileKey {
        id: id.to_string(),
        key,
        value: before,
    }))
}

fn apply_batch(data: &Data, ops: &[Op]) -> AppResult<Applied> {
    let mut inverses = Vec::with_capacity(ops.len());
    let mut changed = Changed::default();
    for op in ops {
        match apply(data, op) {
            Ok(applied) => {
                inverses.push(applied.inverse);
                changed = changed.and(applied.changed);
            }
            Err(e) => {
                for inverse in inverses.iter().rev() {
                    if let Err(undo_error) = apply(data, inverse) {
                        log::error!("Couldn't take back part of a failed change: {undo_error}");
                    }
                }
                return Err(e);
            }
        }
    }
    inverses.reverse();
    Ok(Applied {
        inverse: Op::Batch(inverses),
        changed,
    })
}

fn replace_file(
    dir: &Path,
    before: Option<&FileObject>,
    after: Option<&FileObject>,
) -> AppResult<()> {
    let current = match file::read(dir) {
        FileRead::Found(object) => Some(object),
        FileRead::Missing => None,
        FileRead::Broken(_) => return Err(changed_on_disk(dir)),
    };
    if current.as_ref() != before {
        return Err(changed_on_disk(dir));
    }
    match after {
        Some(object) => file::write(dir, object),
        None => file::remove(dir),
    }
}

fn changed_on_disk(dir: &Path) -> AppError {
    AppError::InvalidInput(format!(
        "{} in {} changed since; it is left as it is",
        file::FILE_NAME,
        dir.display()
    ))
}

pub(super) fn project_not_found(id: &str) -> AppError {
    AppError::NotFound(format!("No project with the id “{id}”"))
}

fn settings_change(inverse: Op) -> Applied {
    Applied {
        inverse,
        changed: Changed {
            settings: true,
            projects: false,
        },
    }
}

fn projects_change(inverse: Op) -> Applied {
    Applied {
        inverse,
        changed: Changed {
            settings: false,
            projects: true,
        },
    }
}
