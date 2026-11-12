//! `AppState`: what Tauri manages for the commands, opened from the config folder.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::data::Data;
use crate::devenv::{self, DevEnv};
use crate::error::AppResult;
use crate::launch::LaunchQueue;
use crate::lock::MutexExt;
use crate::run::Runner;
use crate::run::system::{self, System};
use crate::stats;
use crate::view_state::View;

pub struct AppState {
    pub launches: LaunchQueue,
    pub data: Data,
    pub system: Arc<dyn System>,
    /// Holds terminal scripts and sessions.
    terminal_folder: PathBuf,
    /// The project the main window shows, as its page reports it; the palette and the tray act on it.
    current: Mutex<Option<String>>,
    pub devenv: DevEnv,
    pub stats: stats::Cache,
    /// The main window's frame and sidebar filter, kept for the next start (`view.json`).
    pub view: View,
}

impl AppState {
    /// Debug builds start from the demo seed when the folder holds no projects list yet (ADR 0022).
    pub fn open(config_dir: &Path) -> AppResult<Self> {
        #[cfg(debug_assertions)]
        crate::dev::seed_first_run(config_dir)?;
        let system = system::system();
        Ok(Self {
            launches: LaunchQueue::default(),
            data: Data::open(config_dir),
            devenv: DevEnv::new(
                devenv::tools::tools(system.clone()),
                Box::new(devenv::commands::announce_change),
            ),
            system,
            stats: stats::Cache::default(),
            terminal_folder: config_dir.join("terminal"),
            current: Mutex::new(None),
            view: View::open(config_dir),
        })
    }

    pub fn current_project(&self) -> Option<String> {
        self.current.lock_unpoisoned().clone()
    }

    /// Returns whether it changed.
    pub fn set_current_project(&self, id: Option<String>) -> bool {
        let mut current = self.current.lock_unpoisoned();
        let changed = *current != id;
        *current = id;
        changed
    }

    pub fn runner(&self) -> Runner<'_> {
        Runner {
            data: &self.data,
            system: self.system.as_ref(),
            folder: self.terminal_folder.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = temp_config_dir();
        let state = AppState::open(dir.path()).unwrap();
        (dir, state)
    }

    #[test]
    fn no_project_is_current_at_first() {
        let (_dir, state) = state();
        assert_eq!(state.current_project(), None);
    }

    #[test]
    fn setting_the_current_project_reports_whether_it_changed_and_can_be_read_back() {
        let (_dir, state) = state();

        assert!(state.set_current_project(Some("northwind".into())));
        assert_eq!(state.current_project().as_deref(), Some("northwind"));

        assert!(!state.set_current_project(Some("northwind".into())));
        assert_eq!(state.current_project().as_deref(), Some("northwind"));

        assert!(state.set_current_project(Some("acme-website".into())));
        assert_eq!(state.current_project().as_deref(), Some("acme-website"));

        assert!(state.set_current_project(None));
        assert_eq!(state.current_project(), None);

        assert!(!state.set_current_project(None));
    }
}
