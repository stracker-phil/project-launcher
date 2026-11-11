//! The write path through `Data`: every change, its undo and redo, and what ends up on disk.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::ops::{self, Op};
use super::{Changed, Data, MetaEdit};
use crate::error::{AppError, ErrorKind};
use crate::projects::file::FILE_NAME;
use crate::projects::{FileKey, KnownProject, MetaKey, Registry};
use crate::store::{AppSetting, Appearance, Settings};
use crate::test_support::temp_config_dir;

mod config;
mod library;

const SETTINGS_ONLY: Changed = Changed {
    settings: true,
    projects: false,
};
const PROJECTS_ONLY: Changed = Changed {
    settings: false,
    projects: true,
};
const BOTH: Changed = Changed {
    settings: true,
    projects: true,
};

/// A data folder plus a projects folder that doubles as the home folder (`~`). Project ids equal their folder
/// names.
struct Fixture {
    _config: TempDir,
    root: TempDir,
    data: Data,
}

impl Fixture {
    fn new(projects: &[(&str, Option<Value>)], tags: &[&str]) -> Self {
        let config = temp_config_dir();
        let root = tempfile::tempdir().unwrap();
        let data = Data::open_with_home(config.path(), Some(root.path().to_path_buf()));
        let mut known = Vec::new();
        for (slug, file) in projects {
            let dir = root.path().join(slug);
            fs::create_dir(&dir).unwrap();
            if let Some(file) = file {
                let text = serde_json::to_string_pretty(file).unwrap();
                fs::write(dir.join(FILE_NAME), text).unwrap();
            }
            known.push(KnownProject {
                id: slug.to_string(),
                path: dir.to_string_lossy().into_owned(),
                added_at: 1000,
                last_opened_at: None,
            });
        }
        let settings = Settings {
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Settings::default()
        };
        data.reset(settings, Registry { projects: known }).unwrap();
        Self {
            _config: config,
            root,
            data,
        }
    }

    fn dir(&self, slug: &str) -> PathBuf {
        self.root.path().join(slug)
    }

    fn file(&self, slug: &str) -> Value {
        serde_json::from_str(&self.raw(slug)).unwrap()
    }

    fn raw(&self, slug: &str) -> String {
        fs::read_to_string(self.dir(slug).join(FILE_NAME)).unwrap()
    }

    fn write_raw(&self, slug: &str, text: &str) {
        fs::write(self.dir(slug).join(FILE_NAME), text).unwrap();
    }

    fn has_file(&self, slug: &str) -> bool {
        self.dir(slug).join(FILE_NAME).exists()
    }

    fn ids(&self) -> Vec<String> {
        self.data.projects().into_iter().map(|p| p.id).collect()
    }

    fn tags(&self) -> Vec<String> {
        self.data.settings().tags
    }
}

fn undo_label(data: &Data) -> Option<String> {
    data.undo().unwrap().map(|r| r.label)
}

fn redo_label(data: &Data) -> Option<String> {
    data.redo().unwrap().map(|r| r.label)
}

fn undo_error(data: &Data) -> AppError {
    data.undo().err().expect("the undo fails")
}

fn field_of(err: &AppError) -> Option<&str> {
    err.field().map(|f| f.path)
}

fn alpha_file() -> Value {
    json!({
        "name": "Alpha",
        "meta": { "color": "Red", "url": "https://alpha.example.com" },
        "apps": ["editor", "terminal"],
        "custom": { "keep": true }
    })
}

#[test]
fn appearance_changes_and_undo_and_redo_move_it_back_and_forth() {
    let fx = Fixture::new(&[], &[]);
    assert_eq!(
        fx.data.set_appearance(Appearance::Dark).unwrap(),
        SETTINGS_ONLY
    );
    assert_eq!(fx.data.settings().appearance, Appearance::Dark);

    let undone = fx.data.undo().unwrap().unwrap();
    assert_eq!(undone.label, "Change Appearance");
    assert_eq!(undone.changed, SETTINGS_ONLY);
    assert_eq!(fx.data.settings().appearance, Appearance::System);

    let redone = fx.data.redo().unwrap().unwrap();
    assert_eq!(redone.label, "Change Appearance");
    assert_eq!(fx.data.settings().appearance, Appearance::Dark);
}

#[test]
fn the_appearance_it_already_has_changes_nothing_and_is_not_undoable() {
    let fx = Fixture::new(&[], &[]);
    assert_eq!(
        fx.data.set_appearance(Appearance::System).unwrap(),
        Changed::default()
    );
    assert_eq!(undo_label(&fx.data), None);
}

fn app_value(data: &Data, setting: AppSetting) -> String {
    setting.of(&mut data.settings()).clone()
}

#[test]
fn an_app_setting_is_stored_trimmed_in_its_own_field() {
    for (setting, field) in [
        (AppSetting::DefaultEditor, "defaultEditor"),
        (AppSetting::GitClient, "gitClient"),
        (AppSetting::TerminalApp, "terminalApp"),
        (AppSetting::ConfigEditor, "configEditor"),
    ] {
        let fx = Fixture::new(&[], &[]);
        assert_eq!(
            fx.data.set_app(setting, "  Example App  ").unwrap(),
            SETTINGS_ONLY,
            "{field}"
        );
        assert_eq!(app_value(&fx.data, setting), "Example App", "{field}");
        let stored = serde_json::to_value(fx.data.settings()).unwrap();
        assert_eq!(stored[field], "Example App", "{field}");
    }
}

#[test]
fn setting_one_app_leaves_the_others_alone() {
    let fx = Fixture::new(&[], &[]);
    let terminal = app_value(&fx.data, AppSetting::TerminalApp);
    fx.data
        .set_app(AppSetting::DefaultEditor, "Example Editor")
        .unwrap();
    assert_eq!(app_value(&fx.data, AppSetting::GitClient), "");
    assert_eq!(app_value(&fx.data, AppSetting::ConfigEditor), "");
    assert_eq!(app_value(&fx.data, AppSetting::TerminalApp), terminal);
}

#[test]
fn an_app_setting_undo_restores_the_previous_value_under_its_label() {
    let fx = Fixture::new(&[], &[]);
    for (setting, label) in [
        (AppSetting::DefaultEditor, "Change Default Editor"),
        (AppSetting::GitClient, "Change Git Client"),
        (AppSetting::TerminalApp, "Change Terminal App"),
        (AppSetting::ConfigEditor, "Change Config Editor"),
    ] {
        let before = app_value(&fx.data, setting);
        fx.data.set_app(setting, "Example App").unwrap();

        let undone = fx.data.undo().unwrap().unwrap();
        assert_eq!(undone.label, label);
        assert_eq!(undone.changed, SETTINGS_ONLY, "{label}");
        assert_eq!(app_value(&fx.data, setting), before, "{label}");

        let redone = fx.data.redo().unwrap().unwrap();
        assert_eq!(redone.label, label);
        assert_eq!(app_value(&fx.data, setting), "Example App", "{label}");
        fx.data.undo().unwrap();
    }
}

#[test]
fn an_app_setting_it_already_has_changes_nothing_and_is_not_undoable() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_app(AppSetting::GitClient, "Example Git")
        .unwrap();
    fx.data.undo().unwrap();
    fx.data
        .set_app(AppSetting::GitClient, "Example Git")
        .unwrap();

    for value in ["Example Git", "  Example Git "] {
        assert_eq!(
            fx.data.set_app(AppSetting::GitClient, value).unwrap(),
            Changed::default(),
            "{value:?}"
        );
    }
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Git Client"));
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn a_blank_app_setting_clears_it_back_to_the_default() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_app(AppSetting::DefaultEditor, "Example Editor")
        .unwrap();
    assert_eq!(
        fx.data.set_app(AppSetting::DefaultEditor, "   ").unwrap(),
        SETTINGS_ONLY
    );
    assert_eq!(app_value(&fx.data, AppSetting::DefaultEditor), "");
}

#[test]
fn a_shortcut_is_stored_trimmed_and_undo_removes_the_override() {
    let fx = Fixture::new(&[], &[]);
    assert_eq!(
        fx.data
            .set_shortcut("palette.open", Some(" Cmd+K ".into()))
            .unwrap(),
        SETTINGS_ONLY
    );
    assert_eq!(
        fx.data.settings().shortcuts.get("palette.open"),
        Some(&Some("Cmd+K".to_string()))
    );

    undo_label(&fx.data);
    assert!(fx.data.settings().shortcuts.is_empty());

    redo_label(&fx.data);
    assert_eq!(
        fx.data.settings().shortcuts.get("palette.open"),
        Some(&Some("Cmd+K".to_string()))
    );
}

#[test]
fn unbinding_a_shortcut_leaves_the_command_without_a_key_and_undo_restores_the_old_one() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_shortcut("palette.open", Some("Cmd+K".into()))
        .unwrap();

    fx.data.set_shortcut("palette.open", None).unwrap();
    assert_eq!(
        fx.data.settings().shortcuts.get("palette.open"),
        Some(&None)
    );

    undo_label(&fx.data);
    assert_eq!(
        fx.data.settings().shortcuts.get("palette.open"),
        Some(&Some("Cmd+K".to_string()))
    );
    redo_label(&fx.data);
    assert_eq!(
        fx.data.settings().shortcuts.get("palette.open"),
        Some(&None)
    );
}

#[test]
fn resetting_a_shortcut_drops_the_override_and_undo_brings_it_back() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_shortcut("palette.open", Some("Cmd+K".into()))
        .unwrap();

    assert_eq!(
        fx.data.reset_shortcut("palette.open").unwrap(),
        SETTINGS_ONLY
    );
    assert!(fx.data.settings().shortcuts.is_empty());

    let undone = fx.data.undo().unwrap().unwrap();
    assert_eq!(undone.label, "Reset Shortcut");
    assert_eq!(
        fx.data.settings().shortcuts.get("palette.open"),
        Some(&Some("Cmd+K".to_string()))
    );
    redo_label(&fx.data);
    assert!(fx.data.settings().shortcuts.is_empty());
}

#[test]
fn shortcut_changes_that_change_nothing_are_not_undoable() {
    let fx = Fixture::new(&[], &[]);
    assert_eq!(
        fx.data.reset_shortcut("palette.open").unwrap(),
        Changed::default()
    );
    fx.data
        .set_shortcut("palette.open", Some("Cmd+K".into()))
        .unwrap();
    assert_eq!(
        fx.data
            .set_shortcut("palette.open", Some("Cmd+K".into()))
            .unwrap(),
        Changed::default()
    );
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Shortcut"));
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn a_shortcut_without_a_command_or_a_key_is_refused() {
    let fx = Fixture::new(&[], &[]);
    let blank_command = fx
        .data
        .set_shortcut("  ", Some("Cmd+K".into()))
        .unwrap_err();
    assert_eq!(blank_command.kind(), ErrorKind::InvalidInput);
    let blank_reset = fx.data.reset_shortcut("").unwrap_err();
    assert_eq!(blank_reset.kind(), ErrorKind::InvalidInput);
    let blank_key = fx
        .data
        .set_shortcut("palette.open", Some("  ".into()))
        .unwrap_err();
    assert_eq!(field_of(&blank_key), Some("shortcut"));
    assert_eq!(undo_label(&fx.data), None);
}

fn shortcut_of(data: &Data, command: &str) -> Option<Option<String>> {
    data.settings().shortcuts.get(command).cloned()
}

#[test]
fn reassigning_a_shortcut_moves_the_key_and_unbinds_the_command_that_had_it() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_shortcut("palette.open", Some("Cmd+K".into()))
        .unwrap();

    assert_eq!(
        fx.data
            .reassign_shortcut("project.find", " Cmd+K ", "palette.open")
            .unwrap(),
        SETTINGS_ONLY
    );

    assert_eq!(
        shortcut_of(&fx.data, "project.find"),
        Some(Some("Cmd+K".to_string()))
    );
    assert_eq!(shortcut_of(&fx.data, "palette.open"), Some(None));
}

#[test]
fn reassigning_a_shortcut_from_a_command_without_an_override_still_unbinds_it() {
    let fx = Fixture::new(&[], &[]);

    fx.data
        .reassign_shortcut("project.find", "Cmd+K", "palette.open")
        .unwrap();

    assert_eq!(shortcut_of(&fx.data, "palette.open"), Some(None));
    assert_eq!(
        shortcut_of(&fx.data, "project.find"),
        Some(Some("Cmd+K".to_string()))
    );
}

#[test]
fn a_reassignment_is_one_undo_step_labelled_reassign_shortcut() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_shortcut("palette.open", Some("Cmd+K".into()))
        .unwrap();
    fx.data
        .reassign_shortcut("project.find", "Cmd+K", "palette.open")
        .unwrap();

    let undone = fx.data.undo().unwrap().unwrap();
    assert_eq!(undone.label, "Reassign Shortcut");
    assert_eq!(undone.changed, SETTINGS_ONLY);
    assert_eq!(
        shortcut_of(&fx.data, "palette.open"),
        Some(Some("Cmd+K".to_string()))
    );
    assert_eq!(shortcut_of(&fx.data, "project.find"), None);

    let redone = fx.data.redo().unwrap().unwrap();
    assert_eq!(redone.label, "Reassign Shortcut");
    assert_eq!(shortcut_of(&fx.data, "palette.open"), Some(None));
    assert_eq!(
        shortcut_of(&fx.data, "project.find"),
        Some(Some("Cmd+K".to_string()))
    );

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Reassign Shortcut"));
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Shortcut"));
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn reassigning_with_a_blank_command_key_or_source_is_refused_and_changes_nothing() {
    let fx = Fixture::new(&[], &[]);
    let blank_command = fx
        .data
        .reassign_shortcut(" ", "Cmd+K", "palette.open")
        .unwrap_err();
    assert_eq!(blank_command.kind(), ErrorKind::InvalidInput);
    let blank_from = fx
        .data
        .reassign_shortcut("project.find", "Cmd+K", "")
        .unwrap_err();
    assert_eq!(blank_from.kind(), ErrorKind::InvalidInput);
    let blank_key = fx
        .data
        .reassign_shortcut("project.find", "  ", "palette.open")
        .unwrap_err();
    assert_eq!(field_of(&blank_key), Some("shortcut"));

    assert!(fx.data.settings().shortcuts.is_empty());
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn resetting_a_shortcut_taking_it_from_another_command_is_one_undo_step() {
    let fx = Fixture::new(&[], &[]);
    fx.data
        .set_shortcut("palette.open", Some("Cmd+J".into()))
        .unwrap();
    fx.data
        .set_shortcut("project.find", Some("Cmd+K".into()))
        .unwrap();

    assert_eq!(
        fx.data
            .reset_shortcut_taking_from("palette.open", "project.find")
            .unwrap(),
        SETTINGS_ONLY
    );
    assert_eq!(shortcut_of(&fx.data, "palette.open"), None);
    assert_eq!(shortcut_of(&fx.data, "project.find"), Some(None));

    let undone = fx.data.undo().unwrap().unwrap();
    assert_eq!(undone.label, "Reset Shortcut");
    assert_eq!(
        shortcut_of(&fx.data, "palette.open"),
        Some(Some("Cmd+J".to_string()))
    );
    assert_eq!(
        shortcut_of(&fx.data, "project.find"),
        Some(Some("Cmd+K".to_string()))
    );

    let redone = fx.data.redo().unwrap().unwrap();
    assert_eq!(redone.label, "Reset Shortcut");
    assert_eq!(shortcut_of(&fx.data, "palette.open"), None);
    assert_eq!(shortcut_of(&fx.data, "project.find"), Some(None));
}

#[test]
fn resetting_taking_from_the_same_command_acts_as_a_plain_reset() {
    let fx = Fixture::new(&[], &[]);
    assert_eq!(
        fx.data
            .reset_shortcut_taking_from("palette.open", "palette.open")
            .unwrap(),
        Changed::default()
    );
    fx.data
        .set_shortcut("palette.open", Some("Cmd+J".into()))
        .unwrap();

    assert_eq!(
        fx.data
            .reset_shortcut_taking_from("palette.open", "palette.open")
            .unwrap(),
        SETTINGS_ONLY
    );
    assert_eq!(shortcut_of(&fx.data, "palette.open"), None);
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Reset Shortcut"));
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Shortcut"));
}

#[test]
fn resetting_taking_from_with_a_blank_command_is_refused() {
    let fx = Fixture::new(&[], &[]);
    for (command, from) in [(" ", "palette.open"), ("palette.open", "")] {
        let err = fx
            .data
            .reset_shortcut_taking_from(command, from)
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput, "{command:?} {from:?}");
    }
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn reassigning_a_shortcut_to_the_command_that_has_it_acts_as_setting_it() {
    let fx = Fixture::new(&[], &[]);

    assert_eq!(
        fx.data
            .reassign_shortcut("palette.open", " Cmd+J ", "palette.open")
            .unwrap(),
        SETTINGS_ONLY
    );
    assert_eq!(
        shortcut_of(&fx.data, "palette.open"),
        Some(Some("Cmd+J".to_string()))
    );
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Shortcut"));
    assert_eq!(undo_label(&fx.data), None);

    fx.data
        .set_shortcut("palette.open", Some("Cmd+J".into()))
        .unwrap();
    assert_eq!(
        fx.data
            .reassign_shortcut("palette.open", "Cmd+J", "palette.open")
            .unwrap(),
        Changed::default()
    );
}

#[test]
fn adding_a_tag_appends_it_trimmed_and_undo_removes_it() {
    let fx = Fixture::new(&[], &["Client"]);
    assert_eq!(fx.data.add_tag(" Ideas ").unwrap(), SETTINGS_ONLY);
    assert_eq!(fx.tags(), ["Client", "Ideas"]);

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Add Tag"));
    assert_eq!(fx.tags(), ["Client"]);
    redo_label(&fx.data);
    assert_eq!(fx.tags(), ["Client", "Ideas"]);
}

#[test]
fn adding_a_duplicate_or_blank_tag_is_a_tag_field_error() {
    let fx = Fixture::new(&[], &["Client"]);
    for name in ["Client", " Client ", "", "   "] {
        let err = fx.data.add_tag(name).unwrap_err();
        assert_eq!(field_of(&err), Some("tag"), "{name:?}");
    }
    assert_eq!(fx.tags(), ["Client"]);
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn reordering_tags_applies_the_order_and_undo_restores_the_old_one() {
    let fx = Fixture::new(&[], &["Client", "Personal", "Ideas"]);
    let order = vec![
        "Ideas".to_string(),
        "Client".to_string(),
        "Personal".to_string(),
    ];
    assert_eq!(fx.data.set_tag_order(order).unwrap(), SETTINGS_ONLY);
    assert_eq!(fx.tags(), ["Ideas", "Client", "Personal"]);

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Reorder Tags"));
    assert_eq!(fx.tags(), ["Client", "Personal", "Ideas"]);
    redo_label(&fx.data);
    assert_eq!(fx.tags(), ["Ideas", "Client", "Personal"]);
}

#[test]
fn an_order_that_is_not_a_permutation_of_the_tags_is_invalid_input() {
    let fx = Fixture::new(&[], &["Client", "Personal"]);
    let cases: [&[&str]; 4] = [
        &["Client"],
        &["Client", "Personal", "Ideas"],
        &["Client", "Client"],
        &["Client", "Other"],
    ];
    for order in cases {
        let err = fx
            .data
            .set_tag_order(order.iter().map(|t| t.to_string()).collect())
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput, "{order:?}");
    }
    assert_eq!(fx.tags(), ["Client", "Personal"]);
}

#[test]
fn the_order_the_tags_already_have_changes_nothing() {
    let fx = Fixture::new(&[], &["Client", "Personal"]);
    let same = vec!["Client".to_string(), "Personal".to_string()];
    assert_eq!(fx.data.set_tag_order(same).unwrap(), Changed::default());
    assert_eq!(undo_label(&fx.data), None);
}

fn tagged(name: &str, tag: &str) -> Value {
    json!({ "name": name, "meta": { "tag": tag, "color": "Red" }, "extra": [1, 2] })
}

#[test]
fn renaming_a_tag_changes_the_list_and_every_project_file_that_carries_it() {
    let fx = Fixture::new(
        &[
            ("alpha", Some(tagged("Alpha", "Client"))),
            ("beta", Some(tagged("Beta", "Client"))),
            ("gamma", Some(tagged("Gamma", "Personal"))),
        ],
        &["Client", "Personal"],
    );

    assert_eq!(fx.data.rename_tag("Client", "Clients").unwrap(), BOTH);

    assert_eq!(fx.tags(), ["Clients", "Personal"]);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Clients"));
    assert_eq!(fx.file("beta"), tagged("Beta", "Clients"));
    assert_eq!(fx.file("gamma"), tagged("Gamma", "Personal"));

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Rename Tag"));
    assert_eq!(fx.tags(), ["Client", "Personal"]);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Client"));
    assert_eq!(fx.file("beta"), tagged("Beta", "Client"));

    redo_label(&fx.data);
    assert_eq!(fx.tags(), ["Clients", "Personal"]);
    assert_eq!(fx.file("beta"), tagged("Beta", "Clients"));
}

#[test]
fn renaming_a_tag_onto_an_existing_one_merges_them() {
    let fx = Fixture::new(
        &[
            ("alpha", Some(tagged("Alpha", "Client"))),
            ("gamma", Some(tagged("Gamma", "Personal"))),
        ],
        &["Client", "Personal", "Ideas"],
    );

    fx.data.rename_tag("Client", "Personal").unwrap();

    assert_eq!(fx.tags(), ["Personal", "Ideas"]);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Personal"));
    assert_eq!(fx.file("gamma"), tagged("Gamma", "Personal"));

    undo_label(&fx.data);
    assert_eq!(fx.tags(), ["Client", "Personal", "Ideas"]);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Client"));
}

#[test]
fn renaming_a_tag_only_projects_carry_adds_the_new_name_to_the_list() {
    let fx = Fixture::new(&[("alpha", Some(tagged("Alpha", "Orphan")))], &[]);

    fx.data.rename_tag("Orphan", "Adopted").unwrap();

    assert_eq!(fx.tags(), ["Adopted"]);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Adopted"));
    undo_label(&fx.data);
    assert!(fx.tags().is_empty());
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Orphan"));
}

#[test]
fn renaming_a_tag_to_its_own_name_changes_nothing_and_an_unknown_tag_is_not_found() {
    let fx = Fixture::new(&[], &["Client"]);
    assert_eq!(
        fx.data.rename_tag("Client", "Client").unwrap(),
        Changed::default()
    );
    assert_eq!(undo_label(&fx.data), None);

    let missing = fx.data.rename_tag("Nope", "Other").unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::NotFound);
    let blank = fx.data.rename_tag("Client", "  ").unwrap_err();
    assert_eq!(field_of(&blank), Some("tag"));
    assert_eq!(fx.tags(), ["Client"]);
}

#[test]
fn deleting_a_tag_removes_it_from_the_list_and_from_the_files_that_carry_it() {
    let fx = Fixture::new(
        &[
            ("alpha", Some(tagged("Alpha", "Client"))),
            ("gamma", Some(tagged("Gamma", "Personal"))),
        ],
        &["Client", "Personal"],
    );

    assert_eq!(fx.data.delete_tag("Client").unwrap(), BOTH);

    assert_eq!(fx.tags(), ["Personal"]);
    assert_eq!(
        fx.file("alpha"),
        json!({ "name": "Alpha", "meta": { "color": "Red" }, "extra": [1, 2] })
    );
    assert_eq!(fx.file("gamma"), tagged("Gamma", "Personal"));

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Delete Tag"));
    assert_eq!(fx.tags(), ["Client", "Personal"]);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Client"));
    redo_label(&fx.data);
    assert_eq!(fx.tags(), ["Personal"]);
}

#[test]
fn deleting_a_tag_that_exists_only_on_projects_clears_it_from_their_files() {
    let fx = Fixture::new(&[("alpha", Some(tagged("Alpha", "Orphan")))], &[]);
    fx.data.delete_tag("Orphan").unwrap();
    assert_eq!(fx.data.project("alpha").unwrap().meta.tag, None);
    undo_label(&fx.data);
    assert_eq!(fx.file("alpha"), tagged("Alpha", "Orphan"));
}

#[test]
fn deleting_an_unknown_tag_is_not_found() {
    let fx = Fixture::new(&[], &["Client"]);
    let err = fx.data.delete_tag("Nope").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn a_folder_without_a_project_file_gets_one_named_after_the_folder() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    fs::create_dir(fx.dir("fresh-folder")).unwrap();

    let (id, changed) = fx
        .data
        .add_project(&fx.dir("fresh-folder").to_string_lossy(), 42)
        .unwrap();

    assert_eq!(changed, PROJECTS_ONLY);
    assert_eq!(fx.file("fresh-folder"), json!({ "name": "fresh-folder" }));
    assert_eq!(fx.ids(), ["alpha".to_string(), id.clone()]);
    let project = fx.data.project(&id).unwrap();
    assert_eq!(project.name, "fresh-folder");
    assert_eq!(project.added_at, 42);
    assert_eq!(project.last_opened_at, None);
    assert!(project.available);
}

#[test]
fn undoing_an_added_project_removes_the_file_it_created_and_redo_creates_it_again() {
    let fx = Fixture::new(&[], &[]);
    fs::create_dir(fx.dir("fresh")).unwrap();
    let (id, _) = fx
        .data
        .add_project(&fx.dir("fresh").to_string_lossy(), 1)
        .unwrap();

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Add Project"));
    assert!(!fx.has_file("fresh"));
    assert!(fx.ids().is_empty());
    assert!(fx.dir("fresh").is_dir());

    redo_label(&fx.data);
    assert_eq!(fx.file("fresh"), json!({ "name": "fresh" }));
    assert_eq!(fx.ids(), [id]);
}

#[test]
fn adding_a_folder_with_a_project_file_leaves_the_file_untouched_also_on_undo() {
    let fx = Fixture::new(&[], &[]);
    fs::create_dir(fx.dir("existing")).unwrap();
    let text = "{\n\"name\":   \"Existing\",   \"meta\": {\"color\":\"Green\"}\n}";
    fx.write_raw("existing", text);

    let (id, _) = fx
        .data
        .add_project(&fx.dir("existing").to_string_lossy(), 1)
        .unwrap();
    assert_eq!(fx.raw("existing"), text);
    assert_eq!(fx.data.project(&id).unwrap().name, "Existing");

    undo_label(&fx.data);
    assert_eq!(fx.raw("existing"), text);
    assert!(fx.ids().is_empty());
}

#[test]
fn adding_a_folder_with_a_broken_project_file_keeps_the_file_and_shows_the_problem() {
    let fx = Fixture::new(&[], &[]);
    fs::create_dir(fx.dir("broken")).unwrap();
    fx.write_raw("broken", "{ nope");

    let (id, _) = fx
        .data
        .add_project(&fx.dir("broken").to_string_lossy(), 1)
        .unwrap();

    assert_eq!(fx.raw("broken"), "{ nope");
    let project = fx.data.project(&id).unwrap();
    assert!(!project.available);
    assert!(project.file_error.is_some());
}

#[test]
fn adding_a_known_folder_again_is_a_path_field_error_also_with_a_trailing_slash() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    let dir = fx.dir("alpha").to_string_lossy().into_owned();
    for path in [dir.clone(), format!("{dir}/"), format!("  {dir}  ")] {
        let err = fx.data.add_project(&path, 1).unwrap_err();
        assert_eq!(field_of(&err), Some("path"), "{path}");
    }
    assert_eq!(fx.ids(), ["alpha"]);
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn adding_something_that_is_not_a_folder_is_a_path_field_error() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    let a_file = fx
        .dir("alpha")
        .join(FILE_NAME)
        .to_string_lossy()
        .into_owned();
    let missing = fx.dir("nowhere").to_string_lossy().into_owned();
    for path in [
        a_file,
        missing,
        "relative/folder".to_string(),
        String::new(),
    ] {
        let err = fx.data.add_project(&path, 1).unwrap_err();
        assert_eq!(field_of(&err), Some("path"), "{path}");
    }
    assert_eq!(fx.ids(), ["alpha"]);
}

#[test]
fn a_leading_tilde_stands_for_the_home_folder_when_adding() {
    let fx = Fixture::new(&[], &[]);
    fs::create_dir(fx.dir("fresh")).unwrap();

    let (id, _) = fx.data.add_project("~/fresh", 1).unwrap();
    assert_eq!(fx.data.project(&id).unwrap().path, "~/fresh");
    assert_eq!(fx.file("fresh"), json!({ "name": "fresh" }));

    let (home_id, _) = fx.data.add_project("~", 1).unwrap();
    assert_eq!(fx.data.project(&home_id).unwrap().path, "~");
    assert!(fx.root.path().join(FILE_NAME).exists());
}

#[test]
fn removing_a_project_keeps_its_folder_and_undo_restores_it_at_the_same_index() {
    let fx = Fixture::new(
        &[
            ("alpha", Some(alpha_file())),
            ("beta", Some(alpha_file())),
            ("gamma", Some(alpha_file())),
        ],
        &[],
    );
    let before = fx.data.projects();

    assert_eq!(fx.data.remove_project("beta").unwrap(), PROJECTS_ONLY);
    assert_eq!(fx.ids(), ["alpha", "gamma"]);
    assert!(fx.has_file("beta"));

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Remove Project"));
    assert_eq!(fx.data.projects(), before);

    redo_label(&fx.data);
    assert_eq!(fx.ids(), ["alpha", "gamma"]);
}

#[test]
fn removing_the_first_project_is_restored_first() {
    let fx = Fixture::new(
        &[("alpha", Some(alpha_file())), ("beta", Some(alpha_file()))],
        &[],
    );
    fx.data.remove_project("alpha").unwrap();
    undo_label(&fx.data);
    assert_eq!(fx.ids(), ["alpha", "beta"]);
}

#[test]
fn removing_an_unknown_project_is_not_found() {
    let fx = Fixture::new(&[], &[]);
    let err = fx.data.remove_project("nope").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn renaming_a_project_keeps_every_other_key_in_its_file() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);

    assert_eq!(
        fx.data.set_project_name("alpha", "  Alpha Two ").unwrap(),
        PROJECTS_ONLY
    );

    let mut expected = alpha_file();
    expected["name"] = json!("Alpha Two");
    assert_eq!(fx.file("alpha"), expected);
    assert_eq!(fx.data.project("alpha").unwrap().name, "Alpha Two");

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Rename Project"));
    assert_eq!(fx.file("alpha"), alpha_file());
    assert_eq!(fx.data.project("alpha").unwrap().name, "Alpha");
    redo_label(&fx.data);
    assert_eq!(fx.file("alpha"), expected);
}

#[test]
fn an_empty_name_removes_the_key_so_the_folder_name_shows() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);

    fx.data.set_project_name("alpha", "   ").unwrap();

    let mut expected = alpha_file();
    expected.as_object_mut().unwrap().remove("name");
    assert_eq!(fx.file("alpha"), expected);
    assert_eq!(fx.data.project("alpha").unwrap().name, "alpha");
    undo_label(&fx.data);
    assert_eq!(fx.file("alpha"), alpha_file());
}

#[test]
fn renaming_to_the_current_name_is_not_a_change() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    assert_eq!(
        fx.data.set_project_name("alpha", "Alpha").unwrap(),
        Changed::default()
    );
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn each_meta_edit_sets_its_key_keeps_the_rest_and_undo_restores_the_file() {
    let cases = [
        (
            MetaEdit::Icon(Some("Bolt".into())),
            "icon",
            json!("Bolt"),
            "Change Icon",
        ),
        (
            MetaEdit::Color(Some("Blue".into())),
            "color",
            json!("Blue"),
            "Change Color",
        ),
        (
            MetaEdit::Tag(Some("Client".into())),
            "tag",
            json!("Client"),
            "Change Tag",
        ),
        (
            MetaEdit::Notes(vec!["one".into(), "two".into()]),
            "notes",
            json!(["one", "two"]),
            "Edit Notes",
        ),
        (
            MetaEdit::Starred(true),
            "starred",
            json!(true),
            "Star Project",
        ),
        (
            MetaEdit::Archived(true),
            "archived",
            json!(true),
            "Archive Project",
        ),
    ];
    for (edit, key, value, label) in cases {
        let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &["Client"]);

        let changed = fx.data.set_project_meta("alpha", edit).unwrap();

        let mut expected = alpha_file();
        expected["meta"][key] = value;
        assert_eq!(changed.projects, true, "{label}");
        assert_eq!(fx.file("alpha"), expected, "{label}");

        let undone = fx.data.undo().unwrap().unwrap();
        assert_eq!(undone.label, label);
        assert_eq!(fx.file("alpha"), alpha_file(), "{label}");

        fx.data.redo().unwrap().unwrap();
        assert_eq!(fx.file("alpha"), expected, "{label}");
    }
}

#[test]
fn unstarring_and_unarchiving_use_their_own_labels() {
    let file = json!({ "meta": { "starred": true, "archived": true } });
    let fx = Fixture::new(&[("alpha", Some(file))], &[]);
    fx.data
        .set_project_meta("alpha", MetaEdit::Starred(false))
        .unwrap();
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Unstar Project"));
    fx.data
        .set_project_meta("alpha", MetaEdit::Archived(false))
        .unwrap();
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Unarchive Project"));
}

#[test]
fn empty_meta_values_remove_their_keys() {
    let file = json!({
        "name": "Alpha",
        "meta": {
            "icon": "Bolt", "color": "Red", "tag": "Client", "notes": ["x"],
            "starred": true, "archived": true, "url": "https://alpha.example.com"
        }
    });
    let fx = Fixture::new(&[("alpha", Some(file))], &["Client"]);

    for edit in [
        MetaEdit::Icon(None),
        MetaEdit::Color(Some("  ".into())),
        MetaEdit::Tag(None),
        MetaEdit::Notes(vec![String::new(), "  ".into()]),
        MetaEdit::Starred(false),
        MetaEdit::Archived(false),
    ] {
        fx.data.set_project_meta("alpha", edit).unwrap();
    }

    assert_eq!(
        fx.file("alpha"),
        json!({ "name": "Alpha", "meta": { "url": "https://alpha.example.com" } })
    );
}

#[test]
fn a_new_tag_joins_the_tag_list_in_the_same_undo_step() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &["Client"]);

    let changed = fx
        .data
        .set_project_meta("alpha", MetaEdit::Tag(Some(" Ideas ".into())))
        .unwrap();

    assert_eq!(changed, BOTH);
    assert_eq!(fx.tags(), ["Client", "Ideas"]);
    assert_eq!(
        fx.data.project("alpha").unwrap().meta.tag.as_deref(),
        Some("Ideas")
    );

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Tag"));
    assert_eq!(fx.tags(), ["Client"]);
    assert_eq!(fx.file("alpha"), alpha_file());
    assert_eq!(undo_label(&fx.data), None);

    redo_label(&fx.data);
    assert_eq!(fx.tags(), ["Client", "Ideas"]);
    assert_eq!(
        fx.data.project("alpha").unwrap().meta.tag.as_deref(),
        Some("Ideas")
    );
}

#[test]
fn a_tag_already_in_the_list_is_not_added_again() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &["Client"]);
    let changed = fx
        .data
        .set_project_meta("alpha", MetaEdit::Tag(Some("Client".into())))
        .unwrap();
    assert_eq!(changed, PROJECTS_ONLY);
    assert_eq!(fx.tags(), ["Client"]);
}

#[test]
fn setting_a_meta_value_the_file_already_has_is_not_a_change() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    for edit in [
        MetaEdit::Color(Some("Red".into())),
        MetaEdit::Icon(None),
        MetaEdit::Starred(false),
        MetaEdit::Notes(vec![]),
    ] {
        assert_eq!(
            fx.data.set_project_meta("alpha", edit.clone()).unwrap(),
            Changed::default(),
            "{edit:?}"
        );
    }
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn changing_a_project_without_a_file_is_not_found_and_creates_no_file() {
    let fx = Fixture::new(&[("bare", None)], &[]);
    let name = fx.data.set_project_name("bare", "Bare").unwrap_err();
    let meta = fx
        .data
        .set_project_meta("bare", MetaEdit::Starred(true))
        .unwrap_err();
    assert_eq!(name.kind(), ErrorKind::NotFound);
    assert_eq!(meta.kind(), ErrorKind::NotFound);
    assert!(!fx.has_file("bare"));
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn changing_a_project_with_a_broken_file_is_invalid_input_and_leaves_the_file_alone() {
    let fx = Fixture::new(&[("broken", Some(alpha_file()))], &[]);
    fx.write_raw("broken", "{ \"name\": ");

    let name = fx.data.set_project_name("broken", "Fixed").unwrap_err();
    let meta = fx
        .data
        .set_project_meta("broken", MetaEdit::Tag(Some("Ideas".into())))
        .unwrap_err();

    assert_eq!(name.kind(), ErrorKind::InvalidInput);
    assert_eq!(meta.kind(), ErrorKind::InvalidInput);
    assert_eq!(fx.raw("broken"), "{ \"name\": ");
    assert!(fx.tags().is_empty());
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn changing_an_unknown_project_is_not_found() {
    let fx = Fixture::new(&[], &[]);
    let name = fx.data.set_project_name("nope", "X").unwrap_err();
    let meta = fx
        .data
        .set_project_meta("nope", MetaEdit::Starred(true))
        .unwrap_err();
    assert_eq!(name.kind(), ErrorKind::NotFound);
    assert_eq!(meta.kind(), ErrorKind::NotFound);
}

#[test]
fn only_the_hundred_latest_changes_can_be_undone() {
    let fx = Fixture::new(&[], &[]);
    for i in 0..101 {
        fx.data.add_tag(&format!("t{i}")).unwrap();
    }
    for _ in 0..100 {
        assert_eq!(undo_label(&fx.data).as_deref(), Some("Add Tag"));
    }
    assert_eq!(undo_label(&fx.data), None);
    assert_eq!(fx.tags(), ["t0"]);
}

#[test]
fn a_new_change_clears_what_was_left_to_redo() {
    let fx = Fixture::new(&[], &[]);
    fx.data.add_tag("first").unwrap();
    undo_label(&fx.data);
    fx.data.add_tag("second").unwrap();

    assert_eq!(redo_label(&fx.data), None);
    assert_eq!(fx.tags(), ["second"]);
}

#[test]
fn a_redone_change_can_be_undone_again() {
    let fx = Fixture::new(&[], &[]);
    fx.data.add_tag("first").unwrap();
    undo_label(&fx.data);
    redo_label(&fx.data);
    assert_eq!(undo_label(&fx.data).as_deref(), Some("Add Tag"));
    assert!(fx.tags().is_empty());
}

#[test]
fn undo_and_redo_with_an_empty_history_do_nothing() {
    let fx = Fixture::new(&[], &[]);
    assert_eq!(undo_label(&fx.data), None);
    assert_eq!(redo_label(&fx.data), None);
}

#[test]
fn an_undo_that_cannot_be_applied_is_dropped_and_the_next_undo_continues() {
    let fx = Fixture::new(&[], &[]);
    fs::create_dir(fx.dir("fresh")).unwrap();
    fx.data.add_tag("earlier").unwrap();
    let (id, _) = fx
        .data
        .add_project(&fx.dir("fresh").to_string_lossy(), 1)
        .unwrap();
    fx.write_raw("fresh", "{ \"name\": \"Edited by hand\" }");

    let err = undo_error(&fx.data);

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert_eq!(fx.raw("fresh"), "{ \"name\": \"Edited by hand\" }");
    assert_eq!(fx.ids(), [id]);

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Add Tag"));
    assert!(fx.tags().is_empty());
    assert_eq!(undo_label(&fx.data), None);
    assert_eq!(redo_label(&fx.data).as_deref(), Some("Add Tag"));
    assert_eq!(redo_label(&fx.data), None);
}

#[test]
fn touching_last_opened_records_the_time_and_is_not_undoable() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    fx.data.set_appearance(Appearance::Dark).unwrap();

    assert_eq!(
        fx.data.touch_last_opened("alpha", 5_000).unwrap(),
        PROJECTS_ONLY
    );
    assert_eq!(
        fx.data.project("alpha").unwrap().last_opened_at,
        Some(5_000)
    );

    assert_eq!(undo_label(&fx.data).as_deref(), Some("Change Appearance"));
    assert_eq!(
        fx.data.project("alpha").unwrap().last_opened_at,
        Some(5_000)
    );
    assert_eq!(undo_label(&fx.data), None);
}

#[test]
fn touching_an_unknown_project_is_not_found() {
    let fx = Fixture::new(&[], &[]);
    let err = fx.data.touch_last_opened("nope", 1).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
fn reset_replaces_settings_and_projects_and_clears_history_and_cache() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &["Client"]);
    fx.data.add_tag("extra").unwrap();
    undo_label(&fx.data);
    fx.data.set_appearance(Appearance::Light).unwrap();
    assert_eq!(fx.data.project("alpha").unwrap().name, "Alpha");
    let mut edited = alpha_file();
    edited["name"] = json!("Edited by hand");
    fx.write_raw("alpha", &edited.to_string());

    let registry = Registry {
        projects: vec![KnownProject {
            id: "alpha".into(),
            path: fx.dir("alpha").to_string_lossy().into_owned(),
            added_at: 7,
            last_opened_at: None,
        }],
    };
    let settings = Settings {
        tags: vec!["Fresh".into()],
        ..Settings::default()
    };
    assert_eq!(fx.data.reset(settings, registry).unwrap(), BOTH);

    assert_eq!(undo_label(&fx.data), None);
    assert_eq!(redo_label(&fx.data), None);
    assert_eq!(fx.tags(), ["Fresh"]);
    assert_eq!(fx.data.settings().appearance, Appearance::System);
    assert_eq!(fx.data.project("alpha").unwrap().name, "Edited by hand");
    assert_eq!(fx.data.project("alpha").unwrap().added_at, 7);
}

#[test]
fn reload_reads_hand_edited_project_files_from_disk_again() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    assert_eq!(fx.data.project("alpha").unwrap().name, "Alpha");
    fx.write_raw("alpha", &json!({ "name": "Edited by hand" }).to_string());
    assert_eq!(fx.data.project("alpha").unwrap().name, "Alpha");

    fx.data.reload();

    assert_eq!(fx.data.project("alpha").unwrap().name, "Edited by hand");
}

#[test]
fn a_change_takes_the_hand_edit_made_since_the_last_read_into_account() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &[]);
    fx.data.projects();
    fx.write_raw("alpha", &json!({ "name": "Hand", "mine": 1 }).to_string());

    fx.data
        .set_project_meta("alpha", MetaEdit::Starred(true))
        .unwrap();

    assert_eq!(
        fx.file("alpha"),
        json!({ "name": "Hand", "mine": 1, "meta": { "starred": true } })
    );
}

#[test]
fn a_failing_op_in_a_batch_takes_back_the_ops_before_it() {
    let fx = Fixture::new(&[("alpha", Some(alpha_file()))], &["Client"]);
    let batch = Op::Batch(vec![
        Op::SetTags(vec!["Changed".into()]),
        Op::SetFileKey {
            id: "alpha".into(),
            key: FileKey::Name,
            value: Some(json!("Renamed")),
        },
        Op::SetFileKey {
            id: "nope".into(),
            key: FileKey::Meta(MetaKey::Tag),
            value: None,
        },
    ]);

    let err = ops::apply(&fx.data, &batch).err().expect("the batch fails");

    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert_eq!(fx.tags(), ["Client"]);
    assert_eq!(fx.file("alpha"), alpha_file());
    assert_eq!(fx.data.project("alpha").unwrap().name, "Alpha");
}
