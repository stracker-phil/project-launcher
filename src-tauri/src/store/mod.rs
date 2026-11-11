//! Files in the config folder: private (0700 folders, 0600 files), app data through `JsonStore`
//! (ADR 0012, ADR 0013).

pub mod json_file;
pub mod json_store;
pub mod settings;

pub use json_store::JsonStore;
pub use settings::{AppSetting, Appearance, DockerManager, Settings, Shortcuts};

use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::Path;

/// Creates `dir` and its parents with mode 0700 and tightens an existing one that others can read.
pub fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    if fs::metadata(dir)?.permissions().mode() & 0o077 != 0 {
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn create_private_dir_makes_nested_dirs_private() {
        let dir = temp_config_dir();
        let nested = dir.path().join("a/b");
        create_private_dir(&nested).unwrap();

        assert_eq!(mode_of(&nested), 0o700);
        assert_eq!(mode_of(&dir.path().join("a")), 0o700);
    }

    #[test]
    fn create_private_dir_tightens_an_existing_open_dir() {
        let dir = temp_config_dir();
        let open = dir.path().join("open");
        fs::create_dir(&open).unwrap();
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();

        create_private_dir(&open).unwrap();

        assert_eq!(mode_of(&open), 0o700);
    }
}
