//! A JSON file kept in memory, repaired on load.

use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use serde::Serialize;
use serde_json::Value;

use super::json_file::JsonFile;
use crate::error::AppResult;
use crate::lock::{MutexExt, RwLockExt};

/// Changes are saved before they reach memory, so a failed save never leaves a change that vanishes on
/// restart.
pub struct JsonStore<T> {
    file: JsonFile,
    value: RwLock<T>,
    /// Serializes writers; readers only wait for the swap, never for the disk.
    gate: Mutex<()>,
}

impl<T: Serialize + Clone + Default> JsonStore<T> {
    /// Loads the file through `repair`; a missing or unparsable file loads `T::default()`. The repaired
    /// value is written back only with the next change.
    pub fn open(path: PathBuf, repair: impl FnOnce(Value) -> T) -> Self {
        let file = JsonFile::new(path);
        let value = file.load().map(repair).unwrap_or_default();
        Self {
            file,
            value: RwLock::new(value),
            gate: Mutex::new(()),
        }
    }

    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.value.read_unpoisoned())
    }

    /// Applies `f` to a copy, saves the copy and only then swaps it in. An error from `f` changes nothing.
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> AppResult<R>) -> AppResult<R> {
        let _gate = self.gate.lock_unpoisoned();
        let mut next = self.value.read_unpoisoned().clone();
        let result = f(&mut next)?;
        self.file.save(&next)?;
        *self.value.write_unpoisoned() = next;
        Ok(result)
    }

    pub fn replace(&self, value: T) -> AppResult<()> {
        self.update(|current| {
            *current = value;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AppError, ErrorKind};
    use crate::test_support::temp_config_dir;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn repair(raw: Value) -> Vec<String> {
        serde_json::from_value(raw).unwrap_or_default()
    }

    fn open(path: &Path) -> JsonStore<Vec<String>> {
        JsonStore::open(path.to_path_buf(), repair)
    }

    fn on_disk(path: &Path) -> Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn a_missing_file_opens_with_defaults_and_is_not_created() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");

        let store = open(&path);

        assert_eq!(store.read(Clone::clone), Vec::<String>::new());
        assert!(!path.exists());
    }

    #[test]
    fn an_existing_file_is_loaded_through_the_repair() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");
        fs::write(&path, r#"["alpha", "beta"]"#).unwrap();

        assert_eq!(open(&path).read(Clone::clone), ["alpha", "beta"]);
    }

    #[test]
    fn invalid_json_opens_with_defaults_and_is_moved_aside() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");
        fs::write(&path, "[oops").unwrap();

        let store = open(&path);

        assert_eq!(store.read(Clone::clone), Vec::<String>::new());
        assert!(!path.exists());
        let backups = fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn an_update_changes_memory_and_disk() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");
        let store = open(&path);

        let len = store
            .update(|tags| {
                tags.push("alpha".into());
                Ok(tags.len())
            })
            .unwrap();

        assert_eq!(len, 1);
        assert_eq!(store.read(Clone::clone), ["alpha"]);
        assert_eq!(on_disk(&path), serde_json::json!(["alpha"]));
        assert_eq!(open(&path).read(Clone::clone), ["alpha"]);
    }

    #[test]
    fn a_failing_update_changes_neither_memory_nor_disk() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");
        fs::write(&path, r#"["alpha"]"#).unwrap();
        let store = open(&path);

        let result: AppResult<()> = store.update(|tags| {
            tags.push("beta".into());
            Err(AppError::InvalidInput("nope".into()))
        });

        assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(store.read(Clone::clone), ["alpha"]);
        assert_eq!(fs::read_to_string(&path).unwrap(), r#"["alpha"]"#);
    }

    #[test]
    fn an_update_whose_save_fails_does_not_reach_memory() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");
        fs::write(&path, "[oops").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let store = open(&path);

        let result = store.update(|tags| {
            tags.push("beta".into());
            Ok(())
        });
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(result.unwrap_err().kind(), ErrorKind::Io);
        assert_eq!(store.read(Clone::clone), Vec::<String>::new());
        assert_eq!(fs::read_to_string(&path).unwrap(), "[oops");
    }

    #[test]
    fn replace_swaps_the_whole_value_and_saves_it() {
        let dir = temp_config_dir();
        let path = dir.path().join("tags.json");
        fs::write(&path, r#"["alpha"]"#).unwrap();
        let store = open(&path);

        store.replace(vec!["x".into(), "y".into()]).unwrap();

        assert_eq!(store.read(Clone::clone), ["x", "y"]);
        assert_eq!(on_disk(&path), serde_json::json!(["x", "y"]));
    }

    #[test]
    fn the_saved_file_is_private() {
        let dir = temp_config_dir();
        let path = dir.path().join("sub/tags.json");
        let store = open(&path);

        store.replace(vec!["alpha".into()]).unwrap();

        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
    }
}
