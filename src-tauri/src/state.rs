//! `AppState`: what Tauri manages for the commands, opened from the config folder.

use std::path::Path;

use crate::data::Data;
use crate::error::AppResult;
use crate::launch::LaunchQueue;

pub struct AppState {
    pub launches: LaunchQueue,
    pub data: Data,
}

impl AppState {
    /// Debug builds start from the demo seed when the folder holds no projects list yet (ADR 0022).
    pub fn open(config_dir: &Path) -> AppResult<Self> {
        #[cfg(debug_assertions)]
        crate::dev::seed_first_run(config_dir)?;
        Ok(Self {
            launches: LaunchQueue::default(),
            data: Data::open(config_dir),
        })
    }
}
