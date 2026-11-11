//! Adding folders as projects, the website and repository keys, and the star/archive exclusion.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use tempfile::TempDir;

use crate::data::{AddedProjects, Changed, Data, MetaEdit};
use crate::error::ErrorKind;
use crate::projects::file::FILE_NAME;
use crate::projects::{KnownProject, Registry};
use crate::store::Settings;
use crate::test_support::temp_config_dir;

const PROJECTS_ONLY: Changed = Changed {
    settings: false,
    projects: true,
};

/// A data folder and a projects folder that doubles as the home folder. Listed projects have the id of their
/// folder name.
struct Fx {
    _config: TempDir,
    root: TempDir,
    data: Data,
}

impl Fx {
    fn new(listed: &[(&str, Option<Value>)]) -> Self {
        let config = temp_config_dir();
        let root = tempfile::tempdir().unwrap();
        let data = Data::open_with_home(config.path(), Some(root.path().to_path_buf()));
        let mut known = Vec::new();
        for (slug, file) in listed {
            let dir = root.path().join(slug);
            fs::create_dir(&dir).unwrap();
            if let Some(file) = file {
                fs::write(
                    dir.join(FILE_NAME),
                    serde_json::to_string_pretty(file).unwrap(),
                )
                .unwrap();
            }
            known.push(KnownProject {
                id: slug.to_string(),
                path: dir.to_string_lossy().into_owned(),
                added_at: 1000,
                last_opened_at: None,
            });
        }
        data.reset(Settings::default(), Registry { projects: known })
            .unwrap();
        Self {
            _config: config,
            root,
            data,
        }
    }

    fn folder(&self, slug: &str, file: Option<Value>) -> String {
        let dir = self.root.path().join(slug);
        fs::create_dir(&dir).unwrap();
        if let Some(file) = file {
            fs::write(
                dir.join(FILE_NAME),
                serde_json::to_string_pretty(&file).unwrap(),
            )
            .unwrap();
        }
        dir.to_string_lossy().into_owned()
    }

    fn dir(&self, slug: &str) -> PathBuf {
        self.root.path().join(slug)
    }

    fn file(&self, slug: &str) -> Value {
        let text = fs::read_to_string(self.dir(slug).join(FILE_NAME)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn raw(&self, slug: &str) -> String {
        fs::read_to_string(self.dir(slug).join(FILE_NAME)).unwrap()
    }

    fn ids(&self) -> Vec<String> {
        self.data.projects().into_iter().map(|p| p.id).collect()
    }

    fn undo_label(&self) -> Option<String> {
        self.data.undo().unwrap().map(|r| r.label)
    }
}

#[test]
fn adding_several_folders_is_one_undo_step_named_add_projects() {
    let fx = Fx::new(&[]);
    let one = fx.folder("one", None);
    let two = fx.folder("two", None);

    let (added, changed) = fx.data.add_projects(&[one, two], 5000).unwrap();

    assert_eq!(changed, PROJECTS_ONLY);
    assert_eq!(added.added.len(), 2);
    assert!(added.existing.is_empty());
    assert_eq!(fx.ids(), added.added);
    assert_eq!(fx.undo_label().as_deref(), Some("Add Projects"));
    assert!(fx.ids().is_empty());
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn adding_one_folder_is_named_add_project() {
    let fx = Fx::new(&[]);
    let one = fx.folder("one", None);

    let (added, _) = fx.data.add_projects(&[one], 5000).unwrap();

    assert_eq!(added.added.len(), 1);
    assert_eq!(fx.undo_label().as_deref(), Some("Add Project"));
}

#[test]
fn added_projects_carry_the_given_time_and_were_never_opened() {
    let fx = Fx::new(&[]);
    let one = fx.folder("one", None);

    fx.data.add_projects(&[one], 5000).unwrap();

    let project = fx.data.projects().remove(0);
    assert_eq!((project.added_at, project.last_opened_at), (5000, None));
}

#[test]
fn listed_folders_come_back_as_existing_and_only_the_new_ones_are_added() {
    let fx = Fx::new(&[("listed", Some(json!({"name": "Listed"})))]);
    let listed = fx.dir("listed").to_string_lossy().into_owned();
    let fresh = fx.folder("fresh", None);

    let (added, changed) = fx.data.add_projects(&[listed, fresh], 5000).unwrap();

    assert_eq!(changed, PROJECTS_ONLY);
    assert_eq!(added.existing, ["listed"]);
    assert_eq!(added.added.len(), 1);
    assert_eq!(fx.ids(), ["listed".to_string(), added.added[0].clone()]);
    assert_eq!(fx.undo_label().as_deref(), Some("Add Project"));
}

#[test]
fn a_folder_given_twice_is_added_once() {
    let fx = Fx::new(&[]);
    let one = fx.folder("one", None);

    let (added, _) = fx
        .data
        .add_projects(&[one.clone(), format!("{one}/")], 5000)
        .unwrap();

    assert_eq!(added.added.len(), 1);
    assert!(added.existing.is_empty());
    assert_eq!(fx.ids().len(), 1);
    assert_eq!(fx.undo_label().as_deref(), Some("Add Project"));
}

#[test]
fn a_folder_without_a_project_file_gets_one_with_its_name() {
    let fx = Fx::new(&[]);
    let bare = fx.folder("bare-folder", None);

    fx.data.add_projects(&[bare], 5000).unwrap();

    assert_eq!(fx.file("bare-folder"), json!({ "name": "bare-folder" }));
}

#[test]
fn a_folder_with_a_project_file_keeps_it_untouched() {
    let fx = Fx::new(&[]);
    let text = json!({ "name": "Shop", "meta": { "tag": "Client" }, "custom": [1, 2] });
    let folder = fx.folder("shop", Some(text));
    let before = fx.raw("shop");

    fx.data.add_projects(&[folder], 5000).unwrap();

    assert_eq!(fx.raw("shop"), before);
    assert_eq!(fx.data.projects()[0].name, "Shop");
}

#[test]
fn adding_nothing_new_changes_nothing_and_leaves_no_undo_step() {
    let fx = Fx::new(&[("listed", Some(json!({})))]);
    let listed = fx.dir("listed").to_string_lossy().into_owned();

    let (added, changed) = fx.data.add_projects(&[listed], 5000).unwrap();

    assert_eq!(changed, Changed::default());
    assert_eq!(
        added,
        AddedProjects {
            added: Vec::new(),
            existing: vec!["listed".into()],
        }
    );
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn an_invalid_path_fails_the_whole_call_and_adds_nothing() {
    let fx = Fx::new(&[]);
    let good = fx.folder("good", None);
    let missing = fx.dir("missing").to_string_lossy().into_owned();
    let file_path = fx.dir("a-file");
    fs::write(&file_path, "x").unwrap();

    for bad in [
        missing,
        "relative/folder".to_string(),
        file_path.to_string_lossy().into_owned(),
    ] {
        let err = fx
            .data
            .add_projects(&[good.clone(), bad.clone()], 5000)
            .unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput, "{bad}");
        assert_eq!(err.field().map(|f| f.path), Some("path"), "{bad}");
    }
    assert!(fx.ids().is_empty());
    assert!(!fx.dir("good").join(FILE_NAME).exists());
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn undoing_an_add_removes_the_projects_and_the_files_it_created() {
    let fx = Fx::new(&[]);
    let bare = fx.folder("bare", None);
    let kept = fx.folder("kept", Some(json!({"name": "Kept"})));

    fx.data.add_projects(&[bare, kept], 5000).unwrap();
    assert!(fx.dir("bare").join(FILE_NAME).exists());

    fx.undo_label();

    assert!(fx.ids().is_empty());
    assert!(!fx.dir("bare").join(FILE_NAME).exists());
    assert_eq!(fx.file("kept"), json!({"name": "Kept"}));
}

#[test]
fn add_project_adds_one_folder_and_returns_its_id() {
    let fx = Fx::new(&[]);
    let one = fx.folder("one", None);

    let (id, changed) = fx.data.add_project(&one, 5000).unwrap();

    assert_eq!(changed, PROJECTS_ONLY);
    assert_eq!(fx.ids(), [id]);
    assert_eq!(fx.undo_label().as_deref(), Some("Add Project"));
}

#[test]
fn add_project_refuses_a_folder_that_is_already_listed() {
    let fx = Fx::new(&[("listed", Some(json!({})))]);
    let listed = fx.dir("listed").to_string_lossy().into_owned();

    let err = fx.data.add_project(&listed, 5000).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert_eq!(err.field().map(|f| f.path), Some("path"));
    assert_eq!(fx.ids(), ["listed"]);
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn the_website_and_repository_are_stored_under_their_meta_keys_with_their_labels() {
    let fx = Fx::new(&[("p", Some(json!({"name": "P"})))]);

    fx.data
        .set_project_meta("p", MetaEdit::Url(Some(" https://example.com ".into())))
        .unwrap();
    fx.data
        .set_project_meta(
            "p",
            MetaEdit::RepoUrl(Some("https://example.com/repo".into())),
        )
        .unwrap();

    assert_eq!(
        fx.file("p")["meta"],
        json!({ "url": "https://example.com", "repoUrl": "https://example.com/repo" })
    );
    assert_eq!(fx.undo_label().as_deref(), Some("Change Repository"));
    assert_eq!(fx.undo_label().as_deref(), Some("Change Website"));
}

#[test]
fn a_blank_website_or_repository_removes_its_key() {
    for blank in [None, Some("  ".to_string())] {
        let fx = Fx::new(&[(
            "p",
            Some(
                json!({"meta": {"url": "https://example.com", "repoUrl": "https://example.com/r"}}),
            ),
        )]);

        fx.data
            .set_project_meta("p", MetaEdit::Url(blank.clone()))
            .unwrap();
        fx.data
            .set_project_meta("p", MetaEdit::RepoUrl(blank.clone()))
            .unwrap();

        assert_eq!(fx.file("p"), json!({}), "{blank:?}");
    }
}

#[test]
fn a_locked_project_refuses_a_new_website_and_repository() {
    let fx = Fx::new(&[("p", Some(json!({"locked": true})))]);
    let before = fx.raw("p");

    for edit in [
        MetaEdit::Url(Some("https://example.com".into())),
        MetaEdit::RepoUrl(Some("https://example.com/r".into())),
    ] {
        let err = fx.data.set_project_meta("p", edit.clone()).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput, "{edit:?}");
    }
    assert_eq!(fx.raw("p"), before);
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn starring_an_archived_project_unarchives_it_in_one_undo_step() {
    let fx = Fx::new(&[("p", Some(json!({"meta": {"archived": true}})))]);

    fx.data
        .set_project_meta("p", MetaEdit::Starred(true))
        .unwrap();

    assert_eq!(fx.file("p")["meta"], json!({ "starred": true }));
    assert_eq!(fx.undo_label().as_deref(), Some("Star Project"));
    assert_eq!(fx.file("p")["meta"], json!({ "archived": true }));
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn archiving_a_starred_project_unstars_it_in_one_undo_step() {
    let fx = Fx::new(&[("p", Some(json!({"meta": {"starred": true}})))]);

    fx.data
        .set_project_meta("p", MetaEdit::Archived(true))
        .unwrap();

    assert_eq!(fx.file("p")["meta"], json!({ "archived": true }));
    assert_eq!(fx.undo_label().as_deref(), Some("Archive Project"));
    assert_eq!(fx.file("p")["meta"], json!({ "starred": true }));
    assert_eq!(fx.undo_label(), None);
}

#[test]
fn unstarring_and_unarchiving_leave_the_other_flag_alone() {
    let fx = Fx::new(&[(
        "p",
        Some(json!({"meta": {"starred": true, "archived": true}})),
    )]);

    fx.data
        .set_project_meta("p", MetaEdit::Starred(false))
        .unwrap();
    assert_eq!(fx.file("p")["meta"], json!({ "archived": true }));

    fx.data
        .set_project_meta("p", MetaEdit::Starred(true))
        .unwrap();
    fx.data
        .set_project_meta("p", MetaEdit::Archived(false))
        .unwrap();
    assert_eq!(fx.file("p")["meta"], json!({ "starred": true }));
}
