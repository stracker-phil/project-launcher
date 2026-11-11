//! `AppState`: what Tauri manages for the commands, opened from the config folder.

use std::path::Path;

use crate::error::AppResult;
use crate::launch::LaunchQueue;

#[derive(Default)]
pub struct AppState {
    pub launches: LaunchQueue,
}

impl AppState {
    /// The config folder holds the stores from the data step on.
    pub fn open(_config_dir: &Path) -> AppResult<Self> {
        Ok(Self::default())
    }
}
