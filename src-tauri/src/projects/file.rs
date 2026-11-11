//! A project's own `.project-launcher.json` (ADR 0038). The owner edits it too, so Launcher reads it fresh
//! before each change, touches only the keys it changes and keeps every other key as it was. A broken file is
//! left alone (ADR 0013).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};
use crate::store::json_file::write_atomic;

pub const FILE_NAME: &str = ".project-launcher.json";
/// The mode of a file Launcher creates; an existing file keeps its own.
const NEW_FILE_MODE: u32 = 0o644;

pub type FileObject = Map<String, Value>;

/// What reading a project's file found.
#[derive(Debug, Clone, PartialEq)]
pub enum FileRead {
    Found(FileObject),
    Missing,
    /// Unreadable, invalid JSON or not an object; the message says which.
    Broken(String),
}

fn file_path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

pub fn read(dir: &Path) -> FileRead {
    let raw = match fs::read(file_path(dir)) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return FileRead::Missing,
        Err(e) => return FileRead::Broken(format!("{FILE_NAME} can't be read: {e}")),
    };
    match serde_json::from_slice(&raw) {
        Ok(Value::Object(object)) => FileRead::Found(object),
        Ok(_) => FileRead::Broken(format!("{FILE_NAME} must hold a JSON object")),
        Err(e) => FileRead::Broken(format!("{FILE_NAME} isn't valid JSON: {e}")),
    }
}

/// The file's object, for a change; a missing file is never recreated silently (ADR 0038).
pub fn read_for_change(dir: &Path) -> AppResult<FileObject> {
    match read(dir) {
        FileRead::Found(object) => Ok(object),
        FileRead::Missing => Err(AppError::NotFound(format!(
            "{} has no {FILE_NAME}",
            dir.display()
        ))),
        FileRead::Broken(problem) => Err(AppError::InvalidInput(format!(
            "{problem}; fix the file first"
        ))),
    }
}

/// Writes the object with two-space indents and a final newline, as the owner's editor would.
pub fn write(dir: &Path, object: &FileObject) -> AppResult<()> {
    let path = file_path(dir);
    let mode = fs::metadata(&path)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or(NEW_FILE_MODE);
    let mut json = serde_json::to_vec_pretty(object).map_err(std::io::Error::other)?;
    json.push(b'\n');
    write_atomic(&path, &json, mode).map_err(Into::into)
}

pub fn remove(dir: &Path) -> AppResult<()> {
    match fs::remove_file(file_path(dir)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// The `meta` keys Launcher edits, in the file's spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaKey {
    Icon,
    Color,
    Tag,
    Notes,
    Starred,
    Archived,
}

impl MetaKey {
    fn name(self) -> &'static str {
        match self {
            MetaKey::Icon => "icon",
            MetaKey::Color => "color",
            MetaKey::Tag => "tag",
            MetaKey::Notes => "notes",
            MetaKey::Starred => "starred",
            MetaKey::Archived => "archived",
        }
    }
}

/// A key of the file Launcher changes: a top-level key or a key in `meta`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKey {
    Name,
    Apps,
    Scripts,
    Locked,
    Dev,
    Meta(MetaKey),
}

impl FileKey {
    /// The top-level key the value lives under.
    fn top_level(self) -> &'static str {
        match self {
            FileKey::Name => "name",
            FileKey::Apps => "apps",
            FileKey::Scripts => "scripts",
            FileKey::Locked => "locked",
            FileKey::Dev => "dev",
            FileKey::Meta(_) => "meta",
        }
    }

    pub fn get(self, object: &FileObject) -> Option<Value> {
        match self {
            FileKey::Meta(key) => object.get("meta")?.get(key.name()).cloned(),
            _ => object.get(self.top_level()).cloned(),
        }
    }

    /// Sets or (with `None`) removes the key. Keys keep their place in the file; a new key goes last.
    pub fn set(self, object: &mut FileObject, value: Option<Value>) {
        match self {
            FileKey::Meta(key) => set_meta(object, key, value),
            _ => set_or_remove(object, self.top_level(), value),
        }
    }
}

fn set_or_remove(object: &mut FileObject, key: &str, value: Option<Value>) {
    match value {
        Some(value) => {
            object.insert(key.into(), value);
        }
        None => {
            object.shift_remove(key);
        }
    }
}

/// A `meta` left empty is removed; one that isn't an object is replaced only when a value is set.
fn set_meta(object: &mut FileObject, key: MetaKey, value: Option<Value>) {
    if value.is_some() {
        let meta = object
            .entry("meta")
            .or_insert_with(|| Value::Object(Map::new()));
        if !meta.is_object() {
            *meta = Value::Object(Map::new());
        }
    }
    let Some(Value::Object(meta)) = object.get_mut("meta") else {
        return;
    };
    set_or_remove(meta, key.name(), value);
    if meta.is_empty() {
        object.shift_remove("meta");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_config_dir;
    use serde_json::json;

    fn object(value: Value) -> FileObject {
        let Value::Object(object) = value else {
            panic!("not an object");
        };
        object
    }

    fn keys(object: &FileObject) -> Vec<&str> {
        object.keys().map(String::as_str).collect()
    }

    fn write_raw(dir: &Path, content: &str) {
        fs::write(file_path(dir), content).unwrap();
    }

    #[test]
    fn reading_a_valid_object_finds_it() {
        let dir = temp_config_dir();
        write_raw(
            dir.path(),
            r#"{"name": "Northwind", "meta": {"starred": true}}"#,
        );

        assert_eq!(
            read(dir.path()),
            FileRead::Found(object(
                json!({"name": "Northwind", "meta": {"starred": true}})
            ))
        );
    }

    #[test]
    fn reading_a_folder_without_the_file_is_missing() {
        let dir = temp_config_dir();
        assert_eq!(read(dir.path()), FileRead::Missing);
    }

    #[test]
    fn invalid_json_reads_as_broken() {
        let dir = temp_config_dir();
        write_raw(dir.path(), "{ \"name\": ");

        let FileRead::Broken(problem) = read(dir.path()) else {
            panic!("expected Broken");
        };
        assert!(
            problem.starts_with(".project-launcher.json isn't valid JSON"),
            "{problem}"
        );
    }

    #[test]
    fn json_that_is_not_an_object_reads_as_broken() {
        for content in ["[]", "\"text\"", "42", "null"] {
            let dir = temp_config_dir();
            write_raw(dir.path(), content);

            assert_eq!(
                read(dir.path()),
                FileRead::Broken(".project-launcher.json must hold a JSON object".into()),
                "{content}"
            );
        }
    }

    #[test]
    fn a_file_that_cannot_be_read_reads_as_broken() {
        let dir = temp_config_dir();
        fs::create_dir(file_path(dir.path())).unwrap();

        let FileRead::Broken(problem) = read(dir.path()) else {
            panic!("expected Broken");
        };
        assert!(
            problem.starts_with(".project-launcher.json can't be read"),
            "{problem}"
        );
    }

    #[test]
    fn a_change_needs_a_found_file() {
        let dir = temp_config_dir();
        assert_eq!(
            read_for_change(dir.path()).unwrap_err().kind(),
            crate::error::ErrorKind::NotFound
        );

        write_raw(dir.path(), "nope");
        assert_eq!(
            read_for_change(dir.path()).unwrap_err().kind(),
            crate::error::ErrorKind::InvalidInput
        );

        write_raw(dir.path(), r#"{"name": "A"}"#);
        assert_eq!(
            read_for_change(dir.path()).unwrap(),
            object(json!({"name": "A"}))
        );
    }

    #[test]
    fn a_missing_file_is_not_recreated_by_a_failed_change() {
        let dir = temp_config_dir();
        let _ = read_for_change(dir.path());
        assert!(!file_path(dir.path()).exists());
    }

    #[test]
    fn writing_uses_two_space_indents_and_a_final_newline() {
        let dir = temp_config_dir();

        write(
            dir.path(),
            &object(json!({"name": "A", "meta": {"tag": "client"}})),
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(file_path(dir.path())).unwrap(),
            "{\n  \"name\": \"A\",\n  \"meta\": {\n    \"tag\": \"client\"\n  }\n}\n"
        );
    }

    #[test]
    fn a_new_file_gets_mode_0644() {
        let dir = temp_config_dir();

        write(dir.path(), &object(json!({}))).unwrap();

        let mode = fs::metadata(file_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o644);
    }

    #[test]
    fn an_existing_file_keeps_its_mode() {
        let dir = temp_config_dir();
        write_raw(dir.path(), "{}");
        fs::set_permissions(file_path(dir.path()), fs::Permissions::from_mode(0o600)).unwrap();

        write(dir.path(), &object(json!({"name": "A"}))).unwrap();

        let mode = fs::metadata(file_path(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn writing_leaves_no_temp_file_beside_it() {
        let dir = temp_config_dir();
        write(dir.path(), &object(json!({"name": "A"}))).unwrap();
        write(dir.path(), &object(json!({"name": "B"}))).unwrap();

        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, [FILE_NAME]);
    }

    #[test]
    fn removing_deletes_the_file_and_tolerates_a_missing_one() {
        let dir = temp_config_dir();
        write_raw(dir.path(), "{}");

        remove(dir.path()).unwrap();
        remove(dir.path()).unwrap();

        assert!(!file_path(dir.path()).exists());
    }

    #[test]
    fn name_is_read_and_set_at_the_top_level() {
        let mut file = object(json!({"name": "Old"}));
        assert_eq!(FileKey::Name.get(&file), Some(json!("Old")));

        FileKey::Name.set(&mut file, Some(json!("New")));
        assert_eq!(file, object(json!({"name": "New"})));

        FileKey::Name.set(&mut file, None);
        assert_eq!(FileKey::Name.get(&file), None);
        assert!(file.is_empty());
    }

    #[test]
    fn meta_keys_use_the_files_spelling() {
        let mut file = object(json!({}));
        for (key, name) in [
            (MetaKey::Icon, "icon"),
            (MetaKey::Color, "color"),
            (MetaKey::Tag, "tag"),
            (MetaKey::Notes, "notes"),
            (MetaKey::Starred, "starred"),
            (MetaKey::Archived, "archived"),
        ] {
            FileKey::Meta(key).set(&mut file, Some(json!(name)));
            assert_eq!(file["meta"][name], json!(name), "{name}");
            assert_eq!(FileKey::Meta(key).get(&file), Some(json!(name)), "{name}");
        }
    }

    #[test]
    fn a_meta_key_reads_none_without_meta_or_with_a_non_object_meta() {
        for file in [
            json!({}),
            json!({"meta": "text"}),
            json!({"meta": {"icon": "Bolt"}}),
        ] {
            assert_eq!(
                FileKey::Meta(MetaKey::Tag).get(&object(file.clone())),
                None,
                "{file}"
            );
        }
    }

    #[test]
    fn setting_a_meta_key_keeps_unknown_keys_and_their_order() {
        let mut file = object(json!({
            "zeta": 1,
            "name": "Northwind",
            "meta": {"url": "https://example.com", "tag": "old", "custom": [1, 2]},
            "scripts": {"build": "make"},
        }));

        FileKey::Meta(MetaKey::Tag).set(&mut file, Some(json!("client")));
        FileKey::Meta(MetaKey::Starred).set(&mut file, Some(json!(true)));

        assert_eq!(keys(&file), ["zeta", "name", "meta", "scripts"]);
        assert_eq!(
            file["meta"],
            json!({"url": "https://example.com", "tag": "client", "custom": [1, 2], "starred": true})
        );
        assert_eq!(
            keys(file["meta"].as_object().unwrap()),
            ["url", "tag", "custom", "starred"]
        );
        assert_eq!(file["scripts"], json!({"build": "make"}));
    }

    #[test]
    fn setting_a_key_creates_meta_when_the_file_has_none() {
        let mut file = object(json!({"name": "A"}));

        FileKey::Meta(MetaKey::Icon).set(&mut file, Some(json!("Bolt")));

        assert_eq!(file, object(json!({"name": "A", "meta": {"icon": "Bolt"}})));
    }

    #[test]
    fn a_meta_that_is_not_an_object_is_replaced_when_a_value_is_set() {
        let mut file = object(json!({"meta": "broken"}));

        FileKey::Meta(MetaKey::Color).set(&mut file, Some(json!("Red")));

        assert_eq!(file, object(json!({"meta": {"color": "Red"}})));
    }

    #[test]
    fn removing_a_key_from_a_non_object_meta_leaves_it_alone() {
        let mut file = object(json!({"meta": "broken"}));

        FileKey::Meta(MetaKey::Color).set(&mut file, None);

        assert_eq!(file, object(json!({"meta": "broken"})));
    }

    #[test]
    fn removing_a_meta_key_keeps_the_others() {
        let mut file = object(json!({"meta": {"tag": "client", "icon": "Bolt"}}));

        FileKey::Meta(MetaKey::Tag).set(&mut file, None);

        assert_eq!(file, object(json!({"meta": {"icon": "Bolt"}})));
    }

    #[test]
    fn removing_the_last_meta_key_removes_meta() {
        let mut file = object(json!({"name": "A", "meta": {"tag": "client"}}));

        FileKey::Meta(MetaKey::Tag).set(&mut file, None);

        assert_eq!(file, object(json!({"name": "A"})));
    }

    #[test]
    fn the_launch_config_keys_are_read_and_set_at_the_top_level() {
        let mut file = object(json!({"name": "A"}));
        let values = [
            (
                FileKey::Apps,
                "apps",
                json!(["editor", {"preset": "git", "label": "G"}]),
            ),
            (
                FileKey::Scripts,
                "scripts",
                json!([{"label": "Build", "command": "make"}]),
            ),
            (FileKey::Locked, "locked", json!(true)),
            (FileKey::Dev, "dev", json!({"ddev": true})),
        ];
        for (key, name, value) in values {
            assert_eq!(key.get(&file), None, "{name}");

            key.set(&mut file, Some(value.clone()));

            assert_eq!(file[name], value, "{name}");
            assert_eq!(key.get(&file), Some(value), "{name}");
        }
        assert_eq!(keys(&file), ["name", "apps", "scripts", "locked", "dev"]);

        for (key, name) in [
            (FileKey::Apps, "apps"),
            (FileKey::Scripts, "scripts"),
            (FileKey::Locked, "locked"),
            (FileKey::Dev, "dev"),
        ] {
            key.set(&mut file, None);
            assert_eq!(key.get(&file), None, "{name}");
        }
        assert_eq!(file, object(json!({"name": "A"})));
    }

    #[test]
    fn setting_an_existing_top_level_key_keeps_its_place_and_a_new_one_goes_last() {
        let mut file = object(json!({"apps": ["editor"], "zeta": 1, "name": "A"}));

        FileKey::Apps.set(&mut file, Some(json!(["terminal"])));
        FileKey::Name.set(&mut file, Some(json!("B")));
        assert_eq!(keys(&file), ["apps", "zeta", "name"]);

        FileKey::Locked.set(&mut file, Some(json!(true)));
        assert_eq!(keys(&file), ["apps", "zeta", "name", "locked"]);
    }

    #[test]
    fn removing_a_top_level_key_in_the_middle_keeps_the_order_of_the_others() {
        let mut file = object(json!({
            "name": "A",
            "apps": ["editor"],
            "scripts": [],
            "dev": {"docker": true},
            "zeta": 1,
        }));

        FileKey::Apps.set(&mut file, None);
        assert_eq!(keys(&file), ["name", "scripts", "dev", "zeta"]);

        FileKey::Scripts.set(&mut file, None);
        assert_eq!(keys(&file), ["name", "dev", "zeta"]);
    }

    #[test]
    fn removing_a_meta_key_in_the_middle_keeps_the_order_of_the_others() {
        let mut file = object(json!({
            "meta": {"tag": "client", "icon": "Bolt", "notes": ["x"], "url": "https://example.com"},
        }));

        FileKey::Meta(MetaKey::Tag).set(&mut file, None);

        assert_eq!(
            keys(file["meta"].as_object().unwrap()),
            ["icon", "notes", "url"]
        );
    }

    #[test]
    fn removing_an_emptied_meta_in_the_middle_keeps_the_order_of_the_other_keys() {
        let mut file = object(json!({
            "name": "A",
            "meta": {"tag": "client"},
            "apps": ["editor"],
            "scripts": [],
        }));

        FileKey::Meta(MetaKey::Tag).set(&mut file, None);

        assert_eq!(keys(&file), ["name", "apps", "scripts"]);
    }

    #[test]
    fn removing_an_absent_key_from_an_empty_meta_removes_meta_too() {
        let mut file = object(json!({"meta": {}}));

        FileKey::Meta(MetaKey::Tag).set(&mut file, None);

        assert!(file.is_empty());
    }
}
