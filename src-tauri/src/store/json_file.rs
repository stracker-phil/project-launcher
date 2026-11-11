//! One app-owned JSON file: loaded as raw JSON (missing → `None`, unparsable → moved aside), saved
//! atomically with mode 0600 in a private folder.

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde_json::Value;

use super::create_private_dir;
use crate::error::{AppError, AppResult};

pub struct JsonFile {
    path: PathBuf,
    /// Set when a broken file couldn't be moved aside; it is then never saved over.
    poisoned: AtomicBool,
}

impl JsonFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            poisoned: AtomicBool::new(false),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file's JSON; `None` when it is missing, or unparsable and moved aside as
    /// `<name>.<timestamp>.corrupt`.
    pub fn load(&self) -> Option<Value> {
        let reason = match fs::read(&self.path) {
            Ok(raw) => match serde_json::from_slice(&raw) {
                Ok(value) => return Some(value),
                Err(e) => format!("invalid JSON ({e})"),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
            Err(e) => format!("unreadable ({e})"),
        };
        self.quarantine(&reason);
        None
    }

    fn quarantine(&self, reason: &str) {
        let backup = self.backup_path();
        match fs::rename(&self.path, &backup) {
            Ok(()) => log::warn!(
                "{} is {reason}; moved to {}",
                self.path.display(),
                backup.display()
            ),
            Err(e) => {
                log::error!(
                    "{} is {reason} and cannot be moved aside ({e}); it will not be saved over",
                    self.path.display()
                );
                self.poisoned.store(true, Ordering::Relaxed);
            }
        }
    }

    fn backup_path(&self) -> PathBuf {
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
        let mut backup = sibling(&self.path, &format!("{stamp}.corrupt"));
        let mut n = 1;
        while backup.exists() {
            backup = sibling(&self.path, &format!("{stamp}-{n}.corrupt"));
            n += 1;
        }
        backup
    }

    pub fn save<T: Serialize>(&self, value: &T) -> AppResult<()> {
        if self.poisoned.load(Ordering::Relaxed) {
            return Err(AppError::Io(format!(
                "{} could not be read or moved aside, so it is not overwritten",
                self.path.display()
            )));
        }
        if let Some(dir) = self.path.parent() {
            create_private_dir(dir)?;
        }
        let json = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
        write_atomic(&self.path, &json, 0o600)?;
        Ok(())
    }
}

/// `path` with `.suffix` appended to its file name.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

/// Writes a temp file beside `path` and renames it over `path`, so readers never see half a file.
pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> std::io::Result<()> {
    let tmp = sibling(path, &format!("{}.tmp", std::process::id()));
    let written = write_synced(&tmp, bytes, mode).and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

fn write_synced(path: &Path, bytes: &[u8], mode: u32) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_missing_file_loads_nothing_and_stays_missing() {
        let dir = temp_config_dir();
        let file = JsonFile::new(dir.path().join("settings.json"));

        assert_eq!(file.load(), None);
        assert!(names_in(dir.path()).is_empty());
    }

    #[test]
    fn a_valid_file_loads_its_json() {
        let dir = temp_config_dir();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"tags": ["client"]}"#).unwrap();

        assert_eq!(JsonFile::new(&path).load(), Some(json!({"tags": ["client"]})));
    }

    #[test]
    fn invalid_json_is_moved_aside_with_its_content_and_loads_nothing() {
        let dir = temp_config_dir();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ not json").unwrap();

        assert_eq!(JsonFile::new(&path).load(), None);

        let names = names_in(dir.path());
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(
            names[0].starts_with("settings.json.") && names[0].ends_with(".corrupt"),
            "{}",
            names[0]
        );
        assert_eq!(fs::read_to_string(dir.path().join(&names[0])).unwrap(), "{ not json");
    }

    #[test]
    fn a_second_broken_file_in_the_same_second_gets_its_own_backup_name() {
        let dir = temp_config_dir();
        let path = dir.path().join("settings.json");
        let file = JsonFile::new(&path);

        for content in ["first {", "second {"] {
            fs::write(&path, content).unwrap();
            assert_eq!(file.load(), None);
        }

        let mut contents: Vec<String> = names_in(dir.path())
            .iter()
            .map(|n| fs::read_to_string(dir.path().join(n)).unwrap())
            .collect();
        contents.sort();
        assert_eq!(contents, ["first {", "second {"]);
    }

    #[test]
    fn a_broken_file_that_cannot_be_moved_aside_is_never_saved_over() {
        let dir = temp_config_dir();
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ not json").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let file = JsonFile::new(&path);

        let loaded = file.load();
        let saved = file.save(&json!({"tags": []}));
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(loaded, None);
        assert_eq!(saved.unwrap_err().kind(), crate::error::ErrorKind::Io);
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
    }

    #[test]
    fn saving_creates_a_private_folder_and_a_private_file() {
        let dir = temp_config_dir();
        let folder = dir.path().join("data");
        let path = folder.join("projects.json");

        JsonFile::new(&path).save(&json!({"projects": []})).unwrap();

        assert_eq!(mode_of(&folder), 0o700);
        assert_eq!(mode_of(&path), 0o600);
    }

    #[test]
    fn saving_writes_the_value_and_leaves_no_temp_file() {
        let dir = temp_config_dir();
        let path = dir.path().join("settings.json");
        let file = JsonFile::new(&path);

        file.save(&json!({"tags": ["a"]})).unwrap();
        file.save(&json!({"tags": ["b"]})).unwrap();

        assert_eq!(names_in(dir.path()), ["settings.json"]);
        assert_eq!(file.load(), Some(json!({"tags": ["b"]})));
    }

    #[test]
    fn write_atomic_applies_the_mode_and_replaces_the_content() {
        let dir = temp_config_dir();
        let path = dir.path().join("note.txt");

        write_atomic(&path, b"one", 0o640).unwrap();
        write_atomic(&path, b"two", 0o640).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        assert_eq!(mode_of(&path), 0o640);
        assert_eq!(names_in(dir.path()), ["note.txt"]);
    }

    #[test]
    fn a_failed_atomic_write_leaves_no_temp_file() {
        let dir = temp_config_dir();
        let target = dir.path().join("taken");
        fs::create_dir(&target).unwrap();

        assert!(write_atomic(&target, b"x", 0o600).is_err());

        assert_eq!(names_in(dir.path()), ["taken"]);
    }
}
