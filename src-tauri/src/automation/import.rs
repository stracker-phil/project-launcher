//! The one-time import of the prototype's data folder (`projects.json`, `settings.json`). Projects keep their
//! old ids so Daybook keeps working; their name and details move into each project's file (ADR 0038).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::data::ops::Op;
use crate::data::{Changed, Data, Edit};
use crate::error::{AppError, AppResult};
use crate::projects::file::{self, FileObject};
use crate::projects::{FILE_NAME, FileRead, KnownProject};
use crate::store::{AppSetting, DockerManager, Settings};

/// The prototype's details that move into the project file's `meta`.
const META_KEYS: [&str; 6] = ["icon", "color", "tag", "notes", "starred", "archived"];

/// A project of the prototype's `projects.json`.
#[derive(Debug, Clone, PartialEq)]
struct Old {
    id: String,
    path: String,
    added_at: Option<i64>,
    last_opened_at: Option<i64>,
    name: Option<String>,
    meta: Map<String, Value>,
}

/// A folder the import leaves out.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}

/// What the import changes, for the command line to show before `--yes`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub added: Vec<String>,
    /// Folders the list has already.
    pub known: Vec<String>,
    /// Project files written (created or given the details they lack).
    pub files: Vec<String>,
    pub skipped: Vec<Skipped>,
    /// Settings that change, as "Default editor: PhpStorm".
    pub settings: Vec<String>,
    pub tags: Vec<String>,
}

impl Plan {
    fn skip(&mut self, path: &str, reason: impl Into<String>) {
        self.skipped.push(Skipped {
            path: path.into(),
            reason: reason.into(),
        });
    }
}

fn millis(value: Option<&Value>) -> Option<i64> {
    let text = value?.as_str()?;
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|t| t.timestamp_millis())
}

fn read_json(folder: &Path, name: &str) -> AppResult<Value> {
    let path = folder.join(name);
    let text = fs::read(&path)
        .map_err(|e| AppError::NotFound(format!("{} can’t be read: {e}", path.display())))?;
    serde_json::from_slice(&text)
        .map_err(|e| AppError::InvalidInput(format!("{} isn’t valid JSON: {e}", path.display())))
}

fn old_projects(raw: &Value) -> Vec<Old> {
    raw.as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let meta = entry
                .get("meta")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            Some(Old {
                id: entry.get("id")?.as_str()?.to_string(),
                path: entry.get("path")?.as_str()?.to_string(),
                added_at: millis(entry.get("createdAt")),
                last_opened_at: millis(entry.get("lastOpenedAt")),
                name: meta.get("name").and_then(Value::as_str).map(str::to_string),
                meta: META_KEYS
                    .iter()
                    .filter_map(|key| meta.get(*key).map(|v| (key.to_string(), v.clone())))
                    .collect(),
            })
        })
        .collect()
}

/// The project file with the old name and details it lacks; `None` when it has them all.
fn merged(object: &FileObject, old: &Old) -> Option<FileObject> {
    let mut next = object.clone();
    if let Some(name) = &old.name
        && !next.contains_key("name")
    {
        next.insert("name".into(), Value::String(name.clone()));
    }
    let meta = next
        .entry("meta")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(meta) = meta {
        for (key, value) in &old.meta {
            meta.entry(key.clone()).or_insert_with(|| value.clone());
        }
        if meta.is_empty() {
            next.remove("meta");
        }
    }
    (next != *object).then_some(next)
}

/// Ops for each setting the prototype sets to a different value, labelled for the plan.
fn setting_ops(old: &Value, current: &Settings) -> Vec<(String, Op)> {
    let text = |key: &str| {
        old.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    let mut ops = Vec::new();
    for (key, setting, label) in [
        ("defaultEditor", AppSetting::DefaultEditor, "Default editor"),
        ("gitClient", AppSetting::GitClient, "Git client"),
        ("terminalApp", AppSetting::TerminalApp, "Terminal"),
        (
            "configEditor",
            AppSetting::ConfigEditor,
            "Project file editor",
        ),
        (
            "dockerStartCommand",
            AppSetting::DockerStartCommand,
            "Docker start command",
        ),
        (
            "dockerStopCommand",
            AppSetting::DockerStopCommand,
            "Docker stop command",
        ),
    ] {
        if let Some(value) = text(key)
            && setting.get(current) != value
        {
            ops.push((
                format!("{label}: {value}"),
                Op::SetApp(setting, value.into()),
            ));
        }
    }
    if let Some(manager) = old
        .get("dockerManager")
        .and_then(|m| DockerManager::deserialize(m).ok())
        && manager != current.docker_manager
    {
        ops.push((
            format!("Docker app: {}", crate::devenv::manager_label(manager)),
            Op::SetDockerManager(manager),
        ));
    }
    ops
}

/// The tags the prototype has that the settings lack, in its order.
fn new_tags(old: &Value, current: &[String]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in old
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !current.iter().chain(&tags).any(|t| t == tag) {
            tags.push(tag.to_string());
        }
    }
    tags
}

/// What the list already has: its folders, its ids and its length.
struct Known {
    folders: HashSet<String>,
    ids: HashSet<String>,
    next_index: usize,
}

/// Plans one old project: its file gains what it lacks, and a folder the list lacks joins it.
fn import_project(project: Old, known: &mut Known, plan: &mut Plan, ops: &mut Vec<Op>) {
    let dir = Path::new(&project.path);
    if !dir.is_dir() {
        plan.skip(&project.path, "the folder is gone");
        return;
    }
    let before = match file::read(dir) {
        FileRead::Found(object) => Some(object),
        FileRead::Missing => None,
        FileRead::Broken(reason) => {
            plan.skip(&project.path, reason);
            return;
        }
    };
    if let Some(after) = merged(before.as_ref().unwrap_or(&Map::new()), &project) {
        plan.files
            .push(dir.join(FILE_NAME).to_string_lossy().into_owned());
        ops.push(Op::ReplaceFile {
            dir: project.path.clone(),
            before,
            after: Some(after),
        });
    }
    if known.folders.contains(&project.path) {
        plan.known.push(project.path);
        return;
    }
    plan.added.push(project.path.clone());
    let id = if known.ids.contains(&project.id) {
        uuid::Uuid::new_v4().to_string()
    } else {
        project.id
    };
    ops.push(Op::AddProject {
        project: KnownProject {
            id,
            path: project.path,
            added_at: project
                .added_at
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
            last_opened_at: project.last_opened_at,
        },
        index: known.next_index,
    });
    known.next_index += 1;
}

/// Everything the import would do, and the ops that do it.
fn build(data: &Data, folder: &Path) -> AppResult<(Plan, Vec<Op>)> {
    let old = old_projects(&read_json(folder, "projects.json")?);
    // The settings are optional; without them only the projects come over.
    let old_settings = read_json(folder, "settings.json").unwrap_or(Value::Null);
    let projects = data.projects();
    let mut known = Known {
        folders: projects
            .iter()
            .filter_map(|p| data.project_dir(&p.id).ok())
            .collect(),
        ids: projects.iter().map(|p| p.id.clone()).collect(),
        next_index: projects.len(),
    };
    let mut plan = Plan::default();
    let mut ops = Vec::new();
    for project in old {
        import_project(project, &mut known, &mut plan, &mut ops);
    }
    let settings = data.settings();
    for (text, op) in setting_ops(&old_settings, &settings) {
        plan.settings.push(text);
        ops.push(op);
    }
    plan.tags = new_tags(&old_settings, &settings.tags);
    if !plan.tags.is_empty() {
        ops.push(Op::SetTags([settings.tags, plan.tags.clone()].concat()));
    }
    Ok((plan, ops))
}

pub fn plan(data: &Data, folder: &Path) -> AppResult<Plan> {
    build(data, folder).map(|(plan, _)| plan)
}

/// Imports the folder as one undo step ("Import From Project Launcher").
pub fn apply(data: &Data, folder: &Path) -> AppResult<(Plan, Changed)> {
    let mut plan = Plan::default();
    let changed = data.change(|data| {
        let (built, ops) = build(data, folder)?;
        plan = built;
        Ok((!ops.is_empty()).then(|| Edit {
            label: "Import From Project Launcher",
            op: Op::Batch(ops),
        }))
    })?;
    Ok((plan, changed))
}

/// The command line's import: the plan, or with `yes` the import itself. The debug build refuses, since it
/// never sees real data.
pub fn run(data: &Data, folder: &Path, yes: bool) -> AppResult<(Plan, Option<Changed>)> {
    if cfg!(debug_assertions) {
        return Err(AppError::InvalidInput(
            "The debug build never imports real data; use the installed Launcher.".into(),
        ));
    }
    if yes {
        apply(data, folder).map(|(plan, changed)| (plan, Some(changed)))
    } else {
        plan(data, folder).map(|plan| (plan, None))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::error::ErrorKind;
    use crate::projects::Registry;
    use crate::test_support::temp_config_dir;

    const MARCH_FIRST_MILLIS: i64 = 1_709_287_200_000;
    const MARCH_SECOND_MILLIS: i64 = 1_709_337_600_000;

    /// The app's data, a folder of project folders and a stand-in for the prototype's data folder.
    struct Fx {
        _config: TempDir,
        root: TempDir,
        proto: TempDir,
        data: Data,
    }

    impl Fx {
        fn new() -> Self {
            Self::with(Settings::default(), &[])
        }

        /// `known` folders are in the list already, each under an id equal to its folder name.
        fn with(settings: Settings, known: &[&str]) -> Self {
            let config = temp_config_dir();
            let root = tempfile::tempdir().unwrap();
            let proto = tempfile::tempdir().unwrap();
            let data = Data::open_with_home(config.path(), Some(root.path().to_path_buf()));
            let projects = known
                .iter()
                .map(|slug| {
                    let dir = root.path().join(slug);
                    fs::create_dir(&dir).unwrap();
                    KnownProject {
                        id: slug.to_string(),
                        path: dir.to_string_lossy().into_owned(),
                        added_at: 1000,
                        last_opened_at: None,
                    }
                })
                .collect();
            data.reset(settings, Registry { projects }).unwrap();
            Self {
                _config: config,
                root,
                proto,
                data,
            }
        }

        fn path(&self, slug: &str) -> String {
            self.root.path().join(slug).to_string_lossy().into_owned()
        }

        /// Creates the folder if needed and returns its path.
        fn make(&self, slug: &str) -> String {
            fs::create_dir_all(self.root.path().join(slug)).unwrap();
            self.path(slug)
        }

        fn write_file(&self, slug: &str, text: &str) {
            fs::write(self.root.path().join(slug).join(FILE_NAME), text).unwrap();
        }

        fn raw(&self, slug: &str) -> String {
            fs::read_to_string(self.root.path().join(slug).join(FILE_NAME)).unwrap()
        }

        fn file(&self, slug: &str) -> Value {
            serde_json::from_str(&self.raw(slug)).unwrap()
        }

        fn prototype(&self, projects: Value, settings: Option<Value>) {
            fs::write(
                self.proto.path().join("projects.json"),
                projects.to_string(),
            )
            .unwrap();
            if let Some(settings) = settings {
                fs::write(
                    self.proto.path().join("settings.json"),
                    settings.to_string(),
                )
                .unwrap();
            }
        }

        fn plan(&self) -> Plan {
            plan(&self.data, self.proto.path()).unwrap()
        }

        fn apply(&self) -> (Plan, Changed) {
            apply(&self.data, self.proto.path()).unwrap()
        }
    }

    fn entry(id: &str, path: &str, meta: Value) -> Value {
        json!({ "id": id, "path": path, "meta": meta })
    }

    #[test]
    fn a_new_folder_is_added_under_its_old_id_with_its_dates() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        let blog = fx.make("blog");
        fx.prototype(
            json!([
                {
                    "id": "old-1",
                    "path": acme,
                    "createdAt": "2024-03-01T10:00:00Z",
                    "lastOpenedAt": "2024-03-02T00:00:00Z",
                    "meta": { "name": "Acme" }
                },
                entry("old-2", &blog, json!({ "name": "Blog" })),
            ]),
            None,
        );

        let (plan, changed) = fx.apply();

        assert_eq!(plan.added, vec![acme.clone(), blog.clone()]);
        assert!(changed.projects);
        let first = fx.data.project("old-1").unwrap();
        assert_eq!(first.name, "Acme");
        assert_eq!(first.added_at, MARCH_FIRST_MILLIS);
        assert_eq!(first.last_opened_at, Some(MARCH_SECOND_MILLIS));
        assert_eq!(fx.data.project_dir("old-1").unwrap(), acme);
        let second = fx.data.project("old-2").unwrap();
        assert_eq!(second.last_opened_at, None);
        assert_eq!(fx.data.project_dir("old-2").unwrap(), blog);
    }

    #[test]
    fn planning_changes_nothing() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        fx.prototype(
            json!([entry("old-1", &acme, json!({ "name": "Acme" }))]),
            Some(json!({ "gitClient": "Fork", "tags": ["Client"] })),
        );

        fx.plan();

        assert!(fx.data.projects().is_empty());
        assert!(!fx.root.path().join("acme").join(FILE_NAME).exists());
        assert_eq!(fx.data.settings(), Settings::default());
        assert_eq!(fx.data.undo().unwrap().map(|r| r.label), None);
    }

    #[test]
    fn a_folder_the_list_knows_is_listed_as_known_and_not_added_again() {
        let fx = Fx::with(Settings::default(), &["beta"]);
        let beta = fx.path("beta");
        fx.prototype(
            json!([entry("old-b", &beta, json!({ "name": "Beta" }))]),
            None,
        );

        let plan = fx.plan();

        assert_eq!(plan.known, vec![beta.clone()]);
        assert!(plan.added.is_empty());
        fx.apply();
        assert_eq!(fx.data.projects().len(), 1);
        assert!(fx.data.project("old-b").is_err());
        assert_eq!(fx.data.project("beta").unwrap().name, "Beta");
    }

    #[test]
    fn a_missing_folder_is_skipped() {
        let fx = Fx::new();
        let gone = fx.path("gone");
        fx.prototype(json!([entry("old-g", &gone, json!({}))]), None);

        let plan = fx.plan();

        assert_eq!(
            plan.skipped,
            vec![Skipped {
                path: gone,
                reason: "the folder is gone".into()
            }]
        );
        assert!(plan.added.is_empty());
    }

    #[test]
    fn a_folder_with_a_broken_project_file_is_skipped_with_the_reason() {
        let fx = Fx::new();
        let broken = fx.make("broken");
        fx.write_file("broken", "{ not json");
        fx.prototype(
            json!([entry("old-x", &broken, json!({ "name": "X" }))]),
            None,
        );

        let plan = fx.plan();

        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].path, broken);
        assert!(
            plan.skipped[0]
                .reason
                .starts_with(&format!("{FILE_NAME} isn't valid JSON")),
            "{:?}",
            plan.skipped[0].reason
        );
        assert!(plan.added.is_empty());
        assert!(plan.files.is_empty());
        fx.apply();
        assert_eq!(fx.raw("broken"), "{ not json");
    }

    #[test]
    fn a_folder_without_a_project_file_gets_one_with_the_name_and_details() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        fx.prototype(
            json!([entry(
                "old-1",
                &acme,
                json!({
                    "name": "Acme",
                    "icon": "Leaf",
                    "color": "Teal",
                    "tag": "Client",
                    "notes": "call back",
                    "starred": true,
                    "archived": false,
                    "ignored": "not a detail"
                })
            )]),
            None,
        );

        let (plan, _) = fx.apply();

        assert_eq!(plan.files, vec![format!("{acme}/{FILE_NAME}")]);
        assert_eq!(
            fx.file("acme"),
            json!({
                "name": "Acme",
                "meta": {
                    "icon": "Leaf",
                    "color": "Teal",
                    "tag": "Client",
                    "notes": "call back",
                    "starred": true,
                    "archived": false
                }
            })
        );
    }

    #[test]
    fn an_existing_project_file_keeps_its_own_keys_and_only_gains_missing_ones() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        fx.write_file(
            "acme",
            &json!({
                "name": "Own Name",
                "meta": { "color": "Red" },
                "apps": ["editor"]
            })
            .to_string(),
        );
        fx.prototype(
            json!([entry(
                "old-1",
                &acme,
                json!({ "name": "Old Name", "color": "Teal", "tag": "Client", "starred": true })
            )]),
            None,
        );

        fx.apply();

        assert_eq!(
            fx.file("acme"),
            json!({
                "name": "Own Name",
                "meta": { "color": "Red", "tag": "Client", "starred": true },
                "apps": ["editor"]
            })
        );
    }

    #[test]
    fn a_project_file_that_has_everything_is_not_written() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        let text = "{\"name\":   \"Own Name\", \"meta\": {\"tag\": \"Own\"}}";
        fx.write_file("acme", text);
        fx.prototype(
            json!([entry(
                "old-1",
                &acme,
                json!({ "name": "Old Name", "tag": "Other" })
            )]),
            None,
        );

        let (plan, _) = fx.apply();

        assert!(plan.files.is_empty());
        assert_eq!(plan.added, vec![acme]);
        assert_eq!(fx.raw("acme"), text);
    }

    #[test]
    fn only_settings_that_differ_are_listed_and_applied() {
        let fx = Fx::with(
            Settings {
                default_editor: "Nova".into(),
                ..Settings::default()
            },
            &[],
        );
        fx.prototype(
            json!([]),
            Some(json!({
                "defaultEditor": "Nova",
                "gitClient": " Fork ",
                "terminalApp": "Terminal",
                "configEditor": "Zed",
                "dockerManager": "orbstack",
                "dockerStartCommand": "orb start",
                "dockerStopCommand": ""
            })),
        );

        let (plan, changed) = fx.apply();

        assert_eq!(
            plan.settings,
            vec![
                "Git client: Fork",
                "Project file editor: Zed",
                "Docker start command: orb start",
                "Docker app: OrbStack",
            ]
        );
        assert!(changed.settings);
        assert_eq!(
            fx.data.settings(),
            Settings {
                default_editor: "Nova".into(),
                git_client: "Fork".into(),
                config_editor: "Zed".into(),
                docker_manager: DockerManager::OrbStack,
                docker_start_command: "orb start".into(),
                ..Settings::default()
            }
        );
    }

    #[test]
    fn tags_are_merged_with_new_ones_only_and_their_order_kept() {
        let fx = Fx::with(
            Settings {
                tags: vec!["Personal".into(), "Client".into()],
                ..Settings::default()
            },
            &[],
        );
        fx.prototype(
            json!([]),
            Some(json!({ "tags": ["Client", "Archive", "Personal", "Idea"] })),
        );

        let (plan, _) = fx.apply();

        assert_eq!(plan.tags, vec!["Archive", "Idea"]);
        assert_eq!(
            fx.data.settings().tags,
            vec!["Personal", "Client", "Archive", "Idea"]
        );
    }

    #[test]
    fn a_missing_settings_file_is_fine() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        fx.prototype(
            json!([entry("old-1", &acme, json!({ "name": "Acme" }))]),
            None,
        );

        let plan = fx.plan();

        assert_eq!(plan.added, vec![acme]);
        assert!(plan.settings.is_empty());
        assert!(plan.tags.is_empty());
    }

    #[test]
    fn an_id_the_list_already_uses_gets_a_fresh_id() {
        let fx = Fx::with(Settings::default(), &["taken"]);
        let other = fx.make("other");
        fx.prototype(json!([entry("taken", &other, json!({}))]), None);

        fx.apply();

        let projects = fx.data.projects();
        assert_eq!(projects.len(), 2);
        assert_eq!(fx.data.project_dir("taken").unwrap(), fx.path("taken"));
        let added = projects.iter().find(|p| p.id != "taken").unwrap();
        assert_eq!(fx.data.project_dir(&added.id).unwrap(), other);
    }

    #[test]
    fn applying_is_one_undo_step_that_restores_the_list_files_and_settings() {
        let fx = Fx::with(
            Settings {
                tags: vec!["Client".into()],
                ..Settings::default()
            },
            &[],
        );
        let acme = fx.make("acme");
        let blog = fx.make("blog");
        fx.write_file("blog", &json!({ "name": "Own Blog" }).to_string());
        let blog_before = fx.file("blog");
        fx.prototype(
            json!([
                entry("old-1", &acme, json!({ "name": "Acme", "tag": "Client" })),
                entry("old-2", &blog, json!({ "name": "Blog", "tag": "Client" })),
            ]),
            Some(json!({ "gitClient": "Fork", "tags": ["Client", "New"] })),
        );
        let settings_before = fx.data.settings();

        fx.apply();
        assert_eq!(fx.data.projects().len(), 2);
        assert_ne!(fx.file("blog"), blog_before);

        let undone = fx.data.undo().unwrap().unwrap();

        assert_eq!(undone.label, "Import From Project Launcher");
        assert!(fx.data.projects().is_empty());
        assert!(!fx.root.path().join("acme").join(FILE_NAME).exists());
        assert_eq!(fx.file("blog"), blog_before);
        assert_eq!(fx.data.settings(), settings_before);
        assert!(fx.data.undo().unwrap().is_none());
    }

    #[test]
    fn nothing_to_import_changes_nothing_and_leaves_no_undo_step() {
        let fx = Fx::new();
        fx.prototype(json!([]), Some(json!({})));

        let (plan, changed) = fx.apply();

        assert_eq!(plan, Plan::default());
        assert_eq!(changed, Changed::default());
        assert!(fx.data.undo().unwrap().is_none());
    }

    #[test]
    fn a_missing_projects_file_is_not_found() {
        let fx = Fx::new();

        let err = plan(&fx.data, fx.proto.path()).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[test]
    fn a_projects_file_that_is_not_json_is_invalid_input() {
        let fx = Fx::new();
        fs::write(fx.proto.path().join("projects.json"), "[").unwrap();

        let err = apply(&fx.data, fx.proto.path()).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn the_debug_build_refuses_to_import_with_or_without_yes() {
        let fx = Fx::new();
        let acme = fx.make("acme");
        fx.prototype(
            json!([entry("old-1", &acme, json!({ "name": "Acme" }))]),
            None,
        );

        for yes in [false, true] {
            let err = run(&fx.data, fx.proto.path(), yes).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::InvalidInput, "{yes}");
            assert!(
                err.to_string()
                    .contains("The debug build never imports real data"),
                "{yes}"
            );
        }
        assert!(fx.data.projects().is_empty());
        assert!(!fx.root.path().join("acme").join(FILE_NAME).exists());
    }

    #[test]
    fn a_plan_serializes_with_camel_case_fields() {
        let plan = Plan {
            added: vec!["/a".into()],
            known: vec![],
            files: vec![],
            skipped: vec![Skipped {
                path: "/b".into(),
                reason: "the folder is gone".into(),
            }],
            settings: vec![],
            tags: vec![],
        };
        assert_eq!(
            serde_json::to_value(&plan).unwrap(),
            json!({
                "added": ["/a"],
                "known": [],
                "files": [],
                "skipped": [{ "path": "/b", "reason": "the folder is gone" }],
                "settings": [],
                "tags": []
            })
        );
    }
}
