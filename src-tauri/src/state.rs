//! `AppState`: what Tauri manages for the commands, opened from the config folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::data::Data;
use crate::error::AppResult;
use crate::launch::LaunchQueue;
use crate::run::Runner;
use crate::run::system::{self, System};

pub struct AppState {
    pub launches: LaunchQueue,
    pub data: Data,
    pub system: Arc<dyn System>,
    /// Holds terminal scripts and sessions.
    terminal_folder: PathBuf,
}

impl AppState {
    /// Debug builds start from the demo seed when the folder holds no projects list yet (ADR 0022).
    pub fn open(config_dir: &Path) -> AppResult<Self> {
        #[cfg(debug_assertions)]
        crate::dev::seed_first_run(config_dir)?;
        Ok(Self {
            launches: LaunchQueue::default(),
            data: Data::open(config_dir),
            system: system::system(),
            terminal_folder: config_dir.join("terminal"),
        })
    }

    pub fn runner(&self) -> Runner<'_> {
        Runner {
            data: &self.data,
            system: self.system.as_ref(),
            folder: self.terminal_folder.clone(),
        }
    }
}
