//! What the command line asks for by name: a project from a name, folder or id, an action from its label, and
//! the list and description it prints. `import` brings the prototype's projects and settings over.

pub mod import;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::error::{AppError, AppResult};
use crate::projects::Project;
use crate::projects::config::resolve::Action;

/// A project as `launcher list --json` prints it, `path` absolute; Daybook reads `id`, `name` and `archived`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListedProject {
    pub id: String,
    pub name: String,
    pub path: String,
    pub tag: Option<String>,
    pub starred: bool,
    pub archived: bool,
}

/// What `launcher describe` prints.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Description {
    #[serde(flatten)]
    pub project: ListedProject,
    pub url: Option<String>,
    pub repo_url: Option<String>,
    pub file_error: Option<String>,
    /// Label and what it opens.
    pub apps: Vec<(String, String)>,
    /// Label and command.
    pub scripts: Vec<(String, String)>,
}

fn listed(data: &Data, project: Project) -> ListedProject {
    ListedProject {
        path: data.project_dir(&project.id).unwrap_or(project.path),
        id: project.id,
        name: project.name,
        tag: project.meta.tag,
        starred: project.meta.starred,
        archived: project.meta.archived,
    }
}

pub fn list(data: &Data) -> Vec<ListedProject> {
    let mut projects: Vec<ListedProject> = data
        .projects()
        .into_iter()
        .map(|p| listed(data, p))
        .collect();
    projects.sort_by_key(|p| p.name.to_lowercase());
    projects
}

/// What an action opens, in a few words.
fn opens(action: &Action) -> String {
    let command = action
        .command
        .as_deref()
        .map(|c| if c.is_empty() { "Terminal" } else { c });
    action
        .app
        .as_deref()
        .or(action.url.as_deref())
        .or(command)
        .unwrap_or_default()
        .to_owned()
}

pub fn describe(data: &Data, id: &str) -> AppResult<Description> {
    let project = data.project(id)?;
    let (apps, scripts) = match data.project_config(id) {
        Ok(config) => (
            config
                .apps
                .iter()
                .map(|a| (a.label.clone(), opens(a)))
                .collect(),
            config
                .scripts
                .iter()
                .map(|a| (a.label.clone(), a.command.clone().unwrap_or_default()))
                .collect(),
        ),
        Err(_) => Default::default(),
    };
    Ok(Description {
        url: project.meta.url.clone(),
        repo_url: project.meta.repo_url.clone(),
        file_error: project.file_error.clone(),
        project: listed(data, project),
        apps,
        scripts,
    })
}

/// "a, b, c" of the first few names.
fn some_names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.take(5).collect::<Vec<_>>().join(", ")
}

fn folder_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn canonical(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// The project whose folder holds `dir` (the innermost when they nest).
fn project_at(data: &Data, projects: &[Project], dir: &Path) -> Option<String> {
    projects
        .iter()
        .filter_map(|p| {
            let root = canonical(PathBuf::from(data.project_dir(&p.id).ok()?));
            dir.starts_with(&root)
                .then(|| (root.as_os_str().len(), p.id.clone()))
        })
        .max_by_key(|(length, _)| *length)
        .map(|(_, id)| id)
}

/// The first test that matches anything decides: `Ok(Some)` for one match, `Err` with them all for several.
fn first_match<'a, T>(
    items: &'a [T],
    tests: &[&dyn Fn(&T) -> bool],
) -> Result<Option<&'a T>, Vec<&'a T>> {
    for test in tests {
        let matches: Vec<&T> = items.iter().filter(|item| test(item)).collect();
        match matches.as_slice() {
            [] => continue,
            [one] => return Ok(Some(one)),
            _ => return Err(matches),
        }
    }
    Ok(None)
}

/// A project by id, by an absolute folder inside it (the command line resolves `.` and `~` first), or by name:
/// the exact name, then the folder's name, then a part of the name that only one project has.
pub fn resolve(data: &Data, spec: &str) -> AppResult<String> {
    let projects = data.projects();
    if projects.iter().any(|p| p.id == spec) {
        return Ok(spec.into());
    }
    if spec.starts_with('/') {
        return project_at(data, &projects, &canonical(PathBuf::from(spec)))
            .ok_or_else(|| AppError::NotFound(format!("“{spec}” isn’t inside a known project.")));
    }
    let wanted = spec.to_lowercase();
    match first_match(
        &projects,
        &[
            &|p| p.name.to_lowercase() == wanted,
            &|p| folder_name(&p.path) == wanted,
            &|p| p.name.to_lowercase().contains(&wanted),
        ],
    ) {
        Ok(Some(project)) => Ok(project.id.clone()),
        Ok(None) => Err(AppError::NotFound(format!(
            "No project named “{spec}”. List them with: launcher list"
        ))),
        Err(many) => Err(AppError::InvalidInput(format!(
            "“{spec}” matches several projects: {}.",
            some_names(many.iter().map(|p| p.name.as_str()))
        ))),
    }
}

/// An action by its exact label, else by a part only one label has (case doesn't count). `kind` and `project`
/// name them in the error.
pub fn pick<'a>(
    actions: &'a [Action],
    spec: &str,
    kind: &str,
    project: &str,
) -> AppResult<&'a Action> {
    let wanted = spec.to_lowercase();
    if let Some(exact) = actions.iter().find(|a| a.label.to_lowercase() == wanted) {
        return Ok(exact);
    }
    match first_match(actions, &[&|a| a.label.to_lowercase().contains(&wanted)]) {
        Ok(Some(action)) => Ok(action),
        Ok(None) => Err(AppError::NotFound(format!(
            "{project} has no {kind} “{spec}”. Available: {}.",
            if actions.is_empty() {
                "none".into()
            } else {
                some_names(actions.iter().map(|a| a.label.as_str()))
            }
        ))),
        Err(_) => Err(AppError::InvalidInput(format!(
            "“{spec}” matches several {kind}s in {project}."
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::*;
    use crate::projects::{FILE_NAME, KnownProject, Registry};
    use crate::store::Settings;
    use crate::test_support::temp_config_dir;

    /// A projects folder that doubles as the home folder; each project is `(id, folder, project file)`.
    struct Fx {
        _config: TempDir,
        root: TempDir,
        data: Data,
    }

    impl Fx {
        fn new(projects: &[(&str, &str, Option<Value>)]) -> Self {
            let config = temp_config_dir();
            let root = tempfile::tempdir().unwrap();
            let data = Data::open_with_home(config.path(), Some(root.path().to_path_buf()));
            let mut known = Vec::new();
            for (id, folder, file) in projects {
                let dir = root.path().join(folder);
                fs::create_dir_all(&dir).unwrap();
                if let Some(file) = file {
                    fs::write(dir.join(FILE_NAME), file.to_string()).unwrap();
                }
                known.push(KnownProject {
                    id: id.to_string(),
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

        fn path(&self, folder: &str) -> String {
            self.root.path().join(folder).to_string_lossy().into_owned()
        }
    }

    fn named(name: &str) -> Option<Value> {
        Some(json!({ "name": name }))
    }

    fn resolving() -> Fx {
        Fx::new(&[
            ("p-acme", "acme-shop", named("Acme Shop")),
            ("p-blog", "blog", named("Northwind Blog")),
            ("p-docs", "blog-two", named("Northwind Docs")),
            ("p-notes", "notes-app", named("blog")),
            ("p-outer", "outer", named("Outer")),
            ("p-inner", "outer/inner", named("Inner")),
        ])
    }

    fn not_found(err: AppError) -> String {
        match err {
            AppError::NotFound(message) => message,
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    fn invalid_input(err: AppError) -> String {
        match err {
            AppError::InvalidInput(message) => message,
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn a_project_resolves_by_its_id() {
        let fx = resolving();
        assert_eq!(resolve(&fx.data, "p-acme").unwrap(), "p-acme");
    }

    #[test]
    fn a_folder_resolves_to_the_project_that_holds_it() {
        let fx = resolving();
        fs::create_dir_all(fx.root.path().join("acme-shop/src/components")).unwrap();
        for (folder, id) in [
            ("acme-shop", "p-acme"),
            ("acme-shop/src/components", "p-acme"),
        ] {
            assert_eq!(resolve(&fx.data, &fx.path(folder)).unwrap(), id, "{folder}");
        }
    }

    #[test]
    fn a_folder_in_nested_projects_resolves_to_the_innermost() {
        let fx = resolving();
        fs::create_dir_all(fx.root.path().join("outer/inner/deep")).unwrap();
        fs::create_dir_all(fx.root.path().join("outer/other")).unwrap();
        for (folder, id) in [
            ("outer/inner/deep", "p-inner"),
            ("outer/inner", "p-inner"),
            ("outer/other", "p-outer"),
            ("outer", "p-outer"),
        ] {
            assert_eq!(resolve(&fx.data, &fx.path(folder)).unwrap(), id, "{folder}");
        }
    }

    #[test]
    fn a_folder_outside_every_project_is_not_found() {
        let fx = resolving();
        let elsewhere = tempfile::tempdir().unwrap();
        let spec = elsewhere.path().to_string_lossy().into_owned();
        let message = not_found(resolve(&fx.data, &spec).unwrap_err());
        assert_eq!(message, format!("“{spec}” isn’t inside a known project."));
    }

    #[test]
    fn a_name_resolves_ignoring_case() {
        let fx = resolving();
        for spec in ["Acme Shop", "acme shop", "ACME SHOP"] {
            assert_eq!(resolve(&fx.data, spec).unwrap(), "p-acme", "{spec}");
        }
    }

    #[test]
    fn an_exact_name_beats_a_folder_with_that_name() {
        let fx = resolving();
        assert_eq!(resolve(&fx.data, "blog").unwrap(), "p-notes");
    }

    #[test]
    fn a_folder_name_resolves_when_no_name_matches() {
        let fx = resolving();
        assert_eq!(resolve(&fx.data, "Blog-Two").unwrap(), "p-docs");
    }

    #[test]
    fn a_part_of_a_name_resolves_when_only_one_project_has_it() {
        let fx = resolving();
        for (spec, id) in [("shop", "p-acme"), ("DOCS", "p-docs")] {
            assert_eq!(resolve(&fx.data, spec).unwrap(), id, "{spec}");
        }
    }

    #[test]
    fn a_part_of_several_names_lists_the_candidates() {
        let fx = resolving();
        let message = invalid_input(resolve(&fx.data, "northwind").unwrap_err());
        assert_eq!(
            message,
            "“northwind” matches several projects: Northwind Blog, Northwind Docs."
        );
    }

    #[test]
    fn an_unknown_name_points_to_launcher_list() {
        let fx = resolving();
        let message = not_found(resolve(&fx.data, "zzz").unwrap_err());
        assert_eq!(
            message,
            "No project named “zzz”. List them with: launcher list"
        );
    }

    fn scripts_fx() -> Fx {
        Fx::new(&[(
            "p",
            "acme",
            Some(json!({
                "name": "Acme",
                "scripts": [
                    { "label": "Build", "command": "make" },
                    { "label": "Build Docs", "command": "make docs" },
                    { "label": "Deploy", "command": "./deploy" },
                    { "label": "Lint", "command": "lint" }
                ]
            })),
        )])
    }

    #[test]
    fn an_exact_label_beats_a_partial_match_ignoring_case() {
        let fx = scripts_fx();
        let scripts = fx.data.project_config("p").unwrap().scripts;
        for spec in ["build", "BUILD"] {
            let picked = pick(&scripts, spec, "script", "Acme").unwrap();
            assert_eq!(picked.label, "Build", "{spec}");
        }
    }

    #[test]
    fn a_part_of_one_label_picks_that_action() {
        let fx = scripts_fx();
        let scripts = fx.data.project_config("p").unwrap().scripts;
        for (spec, label) in [("dep", "Deploy"), ("BUILD D", "Build Docs")] {
            let picked = pick(&scripts, spec, "script", "Acme").unwrap();
            assert_eq!(picked.label, label, "{spec}");
        }
    }

    #[test]
    fn a_part_of_several_labels_is_refused() {
        let fx = scripts_fx();
        let scripts = fx.data.project_config("p").unwrap().scripts;
        let message = invalid_input(pick(&scripts, "l", "script", "Acme").unwrap_err());
        assert_eq!(message, "“l” matches several scripts in Acme.");
    }

    #[test]
    fn an_unknown_label_lists_what_is_available() {
        let fx = scripts_fx();
        let scripts = fx.data.project_config("p").unwrap().scripts;
        let message = not_found(pick(&scripts, "zzz", "script", "Acme").unwrap_err());
        assert_eq!(
            message,
            "Acme has no script “zzz”. Available: Build, Build Docs, Deploy, Lint."
        );
    }

    #[test]
    fn without_actions_the_available_list_says_none() {
        let message = not_found(pick(&[], "editor", "app", "Acme").unwrap_err());
        assert_eq!(message, "Acme has no app “editor”. Available: none.");
    }

    #[test]
    fn the_list_is_sorted_by_name_ignoring_case_with_absolute_paths() {
        let fx = Fx::new(&[
            ("p-zeta", "zeta", None),
            (
                "p-alpha",
                "alpha",
                Some(json!({ "name": "Alpha", "meta": { "tag": "Client", "starred": true } })),
            ),
            (
                "p-beta",
                "beta",
                Some(json!({ "name": "beta", "meta": { "archived": true } })),
            ),
        ]);

        assert_eq!(
            list(&fx.data),
            vec![
                ListedProject {
                    id: "p-alpha".into(),
                    name: "Alpha".into(),
                    path: fx.path("alpha"),
                    tag: Some("Client".into()),
                    starred: true,
                    archived: false,
                },
                ListedProject {
                    id: "p-beta".into(),
                    name: "beta".into(),
                    path: fx.path("beta"),
                    tag: None,
                    starred: false,
                    archived: true,
                },
                ListedProject {
                    id: "p-zeta".into(),
                    name: "zeta".into(),
                    path: fx.path("zeta"),
                    tag: None,
                    starred: false,
                    archived: false,
                },
            ]
        );
    }

    #[test]
    fn a_listed_project_serializes_with_camel_case_fields() {
        let fx = Fx::new(&[("p", "acme", named("Acme"))]);
        assert_eq!(
            serde_json::to_value(&list(&fx.data)[0]).unwrap(),
            json!({
                "id": "p",
                "name": "Acme",
                "path": fx.path("acme"),
                "tag": null,
                "starred": false,
                "archived": false
            })
        );
    }

    #[test]
    fn describe_lists_apps_with_what_they_open_and_scripts_with_their_commands() {
        let fx = Fx::new(&[(
            "p",
            "acme-shop",
            Some(json!({
                "name": "Acme Shop",
                "meta": {
                    "tag": "Client",
                    "starred": true,
                    "url": "https://example.com",
                    "repoUrl": "https://example.com/acme.git"
                },
                "apps": [
                    { "label": "Figma", "app": "Figma" },
                    { "label": "Docs", "url": "https://example.com/docs" },
                    { "label": "Shell", "command": "" }
                ],
                "scripts": [{ "label": "Build", "command": "make" }]
            })),
        )]);

        assert_eq!(
            describe(&fx.data, "p").unwrap(),
            Description {
                project: ListedProject {
                    id: "p".into(),
                    name: "Acme Shop".into(),
                    path: fx.path("acme-shop"),
                    tag: Some("Client".into()),
                    starred: true,
                    archived: false,
                },
                url: Some("https://example.com".into()),
                repo_url: Some("https://example.com/acme.git".into()),
                file_error: None,
                apps: vec![
                    ("Figma".into(), "Figma".into()),
                    ("Docs".into(), "https://example.com/docs".into()),
                    ("Shell".into(), "Terminal".into()),
                ],
                scripts: vec![("Build".into(), "make".into())],
            }
        );
    }

    #[test]
    fn describing_a_project_without_a_file_reports_the_problem_and_no_actions() {
        let fx = Fx::new(&[("p", "bare", None)]);

        let description = describe(&fx.data, "p").unwrap();

        assert_eq!(description.project.name, "bare");
        assert_eq!(description.project.path, fx.path("bare"));
        assert_eq!(
            description.file_error,
            Some(format!("{FILE_NAME} is missing"))
        );
        assert!(description.apps.is_empty());
        assert!(description.scripts.is_empty());
    }

    #[test]
    fn a_description_serializes_flat_with_camel_case_fields() {
        let fx = Fx::new(&[("p", "bare", None)]);

        assert_eq!(
            serde_json::to_value(describe(&fx.data, "p").unwrap()).unwrap(),
            json!({
                "id": "p",
                "name": "bare",
                "path": fx.path("bare"),
                "tag": null,
                "starred": false,
                "archived": false,
                "url": null,
                "repoUrl": null,
                "fileError": format!("{FILE_NAME} is missing"),
                "apps": [],
                "scripts": []
            })
        );
    }

    #[test]
    fn describing_an_unknown_id_is_not_found() {
        let fx = Fx::new(&[]);
        assert!(matches!(
            describe(&fx.data, "nope"),
            Err(AppError::NotFound(_))
        ));
    }
}
