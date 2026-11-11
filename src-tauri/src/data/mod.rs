//! The app's data: `settings.json`, `projects.json` and each project's `.project-launcher.json` (ADR 0038),
//! with the one write path (ADR 0007): every change is an `Op` applied through `Data::change`, recorded with its
//! inverse on the undo stack (ADR 0015) and announced by the caller (`Changed`).

pub mod commands;
pub mod edit_menu;
mod edits;
pub mod ops;
mod undo;

pub use edits::{AddedProjects, MetaEdit};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::AppResult;
use crate::library::{self, Filter, Library};
use crate::lock::MutexExt;
use crate::projects::config::{self, ProjectConfig};
use crate::projects::{self, FileRead, KnownProject, Project, Registry};
use crate::store::{JsonStore, Settings};
use ops::{Op, apply};
use undo::{Entry, UndoStack};

pub const SETTINGS_FILE: &str = "settings.json";
pub const PROJECTS_FILE: &str = "projects.json";

/// Which reads a write made stale; each maps to an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Changed {
    pub settings: bool,
    pub projects: bool,
}

impl Changed {
    fn and(self, other: Changed) -> Changed {
        Changed {
            settings: self.settings || other.settings,
            projects: self.projects || other.projects,
        }
    }
}

/// A change before it is applied: its label for Undo and its op.
pub struct Edit {
    pub label: &'static str,
    pub op: Op,
}

/// What undo or redo did.
pub struct Reverted {
    pub label: String,
    pub changed: Changed,
}

pub struct Data {
    settings: JsonStore<Settings>,
    registry: JsonStore<Registry>,
    /// Parsed project files by folder; filled on first read, updated by writes, emptied by `reload`.
    files: Mutex<HashMap<String, FileRead>>,
    undo: Mutex<UndoStack>,
    /// Changes apply one at a time.
    writer: Mutex<()>,
    home: Option<PathBuf>,
}

impl Data {
    pub fn open(config_dir: &Path) -> Self {
        Self::open_with_home(config_dir, std::env::home_dir())
    }

    /// `home` is shown as `~` in project paths.
    pub fn open_with_home(config_dir: &Path, home: Option<PathBuf>) -> Self {
        Self {
            settings: JsonStore::open(config_dir.join(SETTINGS_FILE), Settings::repaired),
            registry: JsonStore::open(config_dir.join(PROJECTS_FILE), Registry::repaired),
            files: Mutex::default(),
            undo: Mutex::default(),
            writer: Mutex::new(()),
            home,
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read(Settings::clone)
    }

    /// Every known project in registry order, each with its file read (or taken from the cache).
    pub fn projects(&self) -> Vec<Project> {
        let known = self.registry.read(|r| r.projects.clone());
        known.iter().map(|k| self.view(k)).collect()
    }

    /// What the main window shows for a sidebar filter and a search.
    pub fn library(&self, filter: &Filter, query: &str, now_ms: i64) -> Library {
        library::build(
            &self.projects(),
            &self.settings().tags,
            filter,
            query,
            now_ms,
        )
    }

    pub fn project(&self, id: &str) -> AppResult<Project> {
        let known = self
            .registry
            .read(|r| r.get(id).cloned())
            .ok_or_else(|| ops::project_not_found(id))?;
        Ok(self.view(&known))
    }

    /// The project's launch config, resolved; refused while its file is missing or broken.
    pub fn project_config(&self, id: &str) -> AppResult<ProjectConfig> {
        let dir = self.project_dir(id)?;
        let object = match self.file_in(&dir) {
            FileRead::Found(object) => object,
            FileRead::Missing | FileRead::Broken(_) => {
                projects::file::read_for_change(Path::new(&dir))?
            }
        };
        let folder = Path::new(&dir);
        let settings = self.settings();
        Ok(config::resolve(
            &object,
            &config::Context {
                dir: folder,
                home: self.home.as_deref(),
                settings: &settings,
                is_git_repo: folder.join(".git").exists(),
                uses_claude: folder.join(".claude").is_dir(),
            },
        ))
    }

    /// Forgets the parsed project files, so the next read takes them from disk.
    pub fn reload(&self) {
        self.files.lock_unpoisoned().clear();
    }

    fn view(&self, known: &KnownProject) -> Project {
        projects::view::build(known, &self.file_in(&known.path), self.home.as_deref())
    }

    /// The project file in `dir`, read on first use.
    fn file_in(&self, dir: &str) -> FileRead {
        self.files
            .lock_unpoisoned()
            .entry(dir.to_string())
            .or_insert_with(|| projects::file::read(Path::new(dir)))
            .clone()
    }

    fn project_dir(&self, id: &str) -> AppResult<String> {
        self.registry
            .read(|r| r.get(id).map(|p| p.path.clone()))
            .ok_or_else(|| ops::project_not_found(id))
    }

    fn cache_file(&self, dir: &str, file: FileRead) {
        self.files.lock_unpoisoned().insert(dir.to_string(), file);
    }

    /// The one write path: `build` turns the current data into an edit (`None`: nothing to change), which is
    /// applied and made undoable, all while no other change runs.
    pub fn change(
        &self,
        build: impl FnOnce(&Self) -> AppResult<Option<Edit>>,
    ) -> AppResult<Changed> {
        let _writer = self.writer.lock_unpoisoned();
        let Some(edit) = build(self)? else {
            return Ok(Changed::default());
        };
        let applied = apply(self, &edit.op)?;
        self.undo.lock_unpoisoned().push(Entry {
            label: edit.label.to_string(),
            op: applied.inverse,
        });
        Ok(applied.changed)
    }

    /// Takes back the last change; `None` when there is none. A change that can't be taken back anymore (its
    /// project is gone, its file was edited since) is dropped with the error, so the next undo goes on.
    pub fn undo(&self) -> AppResult<Option<Reverted>> {
        self.revert(UndoStack::take_undo, UndoStack::push_undone)
    }

    pub fn redo(&self) -> AppResult<Option<Reverted>> {
        self.revert(UndoStack::take_redo, UndoStack::push_redone)
    }

    fn revert(
        &self,
        take: fn(&mut UndoStack) -> Option<Entry>,
        keep: fn(&mut UndoStack, Entry),
    ) -> AppResult<Option<Reverted>> {
        let _writer = self.writer.lock_unpoisoned();
        let Some(entry) = take(&mut self.undo.lock_unpoisoned()) else {
            return Ok(None);
        };
        let applied = apply(self, &entry.op)?;
        keep(
            &mut self.undo.lock_unpoisoned(),
            Entry {
                label: entry.label.clone(),
                op: applied.inverse,
            },
        );
        Ok(Some(Reverted {
            label: entry.label,
            changed: applied.changed,
        }))
    }

    /// Records that a project was opened now. Called when a project is launched; not undoable.
    pub fn touch_last_opened(&self, id: &str, now_ms: i64) -> AppResult<Changed> {
        let _writer = self.writer.lock_unpoisoned();
        self.registry.update(|r| {
            let index = r.position(id).ok_or_else(|| ops::project_not_found(id))?;
            r.projects[index].last_opened_at = Some(now_ms);
            Ok(())
        })?;
        Ok(Changed {
            settings: false,
            projects: true,
        })
    }

    /// Replaces all app data (the demo seed): settings, registry, cached files and the undo history.
    pub fn reset(&self, settings: Settings, registry: Registry) -> AppResult<Changed> {
        let _writer = self.writer.lock_unpoisoned();
        self.settings.replace(settings)?;
        self.registry.replace(registry)?;
        self.files.lock_unpoisoned().clear();
        self.undo.lock_unpoisoned().clear();
        Ok(Changed {
            settings: true,
            projects: true,
        })
    }
}

#[cfg(test)]
mod tests;
