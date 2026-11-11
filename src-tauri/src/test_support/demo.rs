//! The demo seed of the debug build (ADR 0022): made-up project folders inside the debug data folder, each
//! with a `.project-launcher.json` in the prototype's format, and the settings and registry that list them.

use std::fs;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::error::AppResult;
use crate::projects::file;
use crate::projects::{KnownProject, Registry};
use crate::store::{Settings, create_private_dir};

/// The folder in the data folder that holds the demo projects.
pub const DEMO_FOLDER: &str = "demo-projects";

const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * 60 * MINUTE;

pub struct Seed {
    pub settings: Settings,
    pub registry: Registry,
}

struct DemoProject {
    slug: &'static str,
    /// The project file; `None` leaves the folder without one.
    file: Option<Value>,
    /// Another file that makes the folder look like a project.
    manifest: Option<(&'static str, Value)>,
    /// More files, by path inside the folder.
    files: &'static [(&'static str, &'static str)],
    /// A git history: each commit's subject and how many days ago it was made, oldest first.
    history: &'static [(&'static str, i64)],
    added_days_ago: i64,
    opened_minutes_ago: Option<i64>,
}

fn projects() -> Vec<DemoProject> {
    vec![
        DemoProject {
            slug: "acme-website",
            file: Some(json!({
                "name": "Acme Website",
                "meta": {
                    "icon": "Globe",
                    "color": "Blue",
                    "tag": "Client",
                    "url": "https://acme.example.com",
                    "repoUrl": "https://git.example.com/acme/website",
                    "notes": ["Relaunch goes live in March", "Staging: https://staging.acme.example.com"],
                    "starred": true
                },
                "dev": { "ddev": true },
                "apps": [
                    "editor",
                    { "preset": "terminal", "label": "Shell", "shortcut": "cmd+shift+t" },
                    "git",
                    "browser",
                    "repoBrowser"
                ],
                "scripts": [
                    { "label": "Build", "command": "npm run build", "icon": "Hammer" },
                    { "label": "Lint", "command": "npm run lint" },
                    { "label": "Export Database", "command": "ddev export-db --file=${dir}/backup.sql.gz" }
                ]
            })),
            manifest: Some((
                "package.json",
                json!({
                    "name": "acme-website",
                    "version": "2.4.0",
                    "private": true,
                    "scripts": { "dev": "vite", "build": "vite build", "lint": "eslint src" },
                    "engines": { "node": ">=20" }
                }),
            )),
            files: &[
                (
                    ".ddev/config.yaml",
                    "name: acme-website\ntype: php\ndatabase:\n  type: mariadb\n  version: \"10.11\"\n",
                ),
                (
                    "index.html",
                    "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"utf-8\" />\n    <title>Acme</title>\n  </head>\n  <body>\n    <div id=\"app\"></div>\n    <script type=\"module\" src=\"/src/main.js\"></script>\n  </body>\n</html>\n",
                ),
                (
                    "src/main.js",
                    "import { header } from \"./components/header.js\";\nimport { pricing } from \"./components/pricing.js\";\nimport \"./style.css\";\n\nconst app = document.querySelector(\"#app\");\napp.append(header(), pricing());\n",
                ),
                (
                    "src/components/header.js",
                    "export function header() {\n  const element = document.createElement(\"header\");\n  element.textContent = \"Acme: tools for the busy workshop\";\n  return element;\n}\n",
                ),
                (
                    "src/components/pricing.js",
                    "const plans = [\n  { name: \"Starter\", price: 9 },\n  { name: \"Team\", price: 29 },\n];\n\nexport function pricing() {\n  const list = document.createElement(\"ul\");\n  for (const plan of plans) {\n    const item = document.createElement(\"li\");\n    item.textContent = plan.name + \": \" + plan.price;\n    list.append(item);\n  }\n  return list;\n}\n",
                ),
                (
                    "src/style.css",
                    "body {\n  margin: 0;\n  font-family: system-ui, sans-serif;\n}\n\nheader {\n  padding: 24px;\n  font-weight: 600;\n}\n",
                ),
                (
                    "web/theme/functions.php",
                    "<?php\n\nadd_action('wp_enqueue_scripts', function () {\n    wp_enqueue_style('acme', get_stylesheet_uri());\n});\n",
                ),
                (
                    "README.md",
                    "# Acme Website\n\nThe marketing site. `npm run dev` starts it, `ddev start` the WordPress backend.\n",
                ),
            ],
            history: &[
                ("Start the relaunch", 80),
                ("Add the header", 74),
                ("Add the pricing table", 66),
                ("Set up DDEV", 58),
                ("Style the header", 45),
                ("Add the theme functions", 33),
                ("Fix the pricing currency", 26),
                ("Tidy the README", 19),
                ("Add the Team plan", 12),
                ("Update the plans", 8),
                ("Polish the styles", 5),
                ("Prepare the March launch", 2),
            ],
            added_days_ago: 120,
            opened_minutes_ago: Some(12),
        },
        DemoProject {
            slug: "orbit-api",
            file: Some(json!({
                "name": "Orbit API",
                "meta": {
                    "icon": "Bolt",
                    "color": "Purple",
                    "tag": "Client",
                    "repoUrl": "https://git.example.com/orbit/api",
                    "notes": "Deploys on Tuesdays\nAsk Sam Example before changing the schema"
                },
                "env": { "APP_ENV": "local", "CLAUDE_CONFIG_DIR": "~/.claude-demo" },
                "dev": { "docker": true },
                "database": "mysql://orbit:demo-password@127.0.0.1:3306/orbit",
                "apps": [
                    "editor",
                    "terminal",
                    "claude",
                    { "label": "API Docs", "url": "https://docs.orbit.example.com", "icon": "Book" },
                    { "label": "Tail Logs", "command": "tail -f ${dir}/storage/logs/app.log", "hiddenStates": ["paused"] }
                ],
                "scripts": [{ "label": "Run Tests", "command": "composer test", "shortcut": "cmd+shift+u" }],
                "stateProviders": { "queue": "cat ${dir}/storage/queue-state" },
                "states": {
                    "queue": {
                        "source": "queue",
                        "label": "Queue",
                        "values": {
                            "running": { "value": "running", "label": "Running", "color": "Green" },
                            "paused": { "value": "paused", "label": "Paused", "color": "Orange" }
                        }
                    }
                }
            })),
            manifest: Some((
                "composer.json",
                json!({
                    "name": "orbit/api",
                    "type": "project",
                    "require": { "php": ">=8.2" },
                    "scripts": { "test": "phpunit", "serve": "php -S localhost:8000 -t public" }
                }),
            )),
            files: &[
                (
                    "public/index.php",
                    "<?php\n\nrequire __DIR__ . '/../vendor/autoload.php';\n\n(new Orbit\\Router())->dispatch($_SERVER['REQUEST_URI']);\n",
                ),
                (
                    "src/Router.php",
                    "<?php\n\nnamespace Orbit;\n\nfinal class Router\n{\n    public function dispatch(string $path): void\n    {\n        match ($path) {\n            '/orders' => (new Controller\\OrderController())->index(),\n            default => http_response_code(404),\n        };\n    }\n}\n",
                ),
                (
                    "src/Controller/OrderController.php",
                    "<?php\n\nnamespace Orbit\\Controller;\n\nfinal class OrderController\n{\n    public function index(): void\n    {\n        header('Content-Type: application/json');\n        echo json_encode(['orders' => []]);\n    }\n}\n",
                ),
                (
                    "tests/RouterTest.php",
                    "<?php\n\nuse PHPUnit\\Framework\\TestCase;\n\nfinal class RouterTest extends TestCase\n{\n    public function testUnknownPathsAreNotFound(): void\n    {\n        $this->assertTrue(true);\n    }\n}\n",
                ),
            ],
            history: &[
                ("Set up the API", 40),
                ("Add the order list", 31),
                ("Route unknown paths to 404", 22),
                ("Add the router test", 15),
                ("Deploy on Tuesdays", 6),
            ],
            added_days_ago: 90,
            opened_minutes_ago: Some(2 * 24 * 60),
        },
        DemoProject {
            slug: "pixel-notes",
            file: Some(json!({
                "name": "Pixel Notes",
                "meta": { "icon": "Pencil", "color": "Green", "tag": "Personal", "starred": true },
                "apps": ["editor", "terminal"],
                "scripts": [{ "label": "Dev Server", "command": "bun run dev" }]
            })),
            manifest: Some((
                "package.json",
                json!({ "name": "pixel-notes", "version": "0.3.1" }),
            )),
            files: &[],
            history: &[],
            added_days_ago: 45,
            opened_minutes_ago: Some(3 * 60),
        },
        DemoProject {
            slug: "garden-planner",
            file: Some(json!({
                "name": "Garden Planner",
                "meta": { "icon": "Leaf", "color": "Yellow", "tag": "Personal" },
                "locked": true,
                "apps": ["editor"]
            })),
            manifest: None,
            files: &[],
            history: &[],
            added_days_ago: 30,
            opened_minutes_ago: None,
        },
        DemoProject {
            slug: "weather-widget",
            file: Some(json!({
                "meta": { "color": "Red", "tag": "Experiments", "notes": ["Try the new chart library"] },
                "apps": ["editor", "terminal"]
            })),
            manifest: None,
            files: &[],
            history: &[],
            added_days_ago: 14,
            opened_minutes_ago: Some(8 * 24 * 60),
        },
        DemoProject {
            slug: "old-portfolio",
            file: Some(json!({
                "name": "Old Portfolio",
                "meta": { "icon": "Book", "color": "Orange", "archived": true, "notes": ["Replaced by the new site"] },
                "apps": ["editor"]
            })),
            manifest: None,
            files: &[],
            history: &[],
            added_days_ago: 400,
            opened_minutes_ago: None,
        },
        DemoProject {
            slug: "scratchpad",
            file: None,
            manifest: None,
            files: &[],
            history: &[],
            added_days_ago: 3,
            opened_minutes_ago: None,
        },
    ]
}

/// Makes `dir` a git repository with `history`: the files in the first commit, then empty ones, each dated that
/// many days back. The owner's git config, hooks and signing stay out; without git it does nothing.
fn commit_history(dir: &Path, history: &[(&str, i64)], now_ms: i64) {
    let git = |args: &[&str], days_ago: i64| {
        let at = format!("@{} +0000", now_ms / 1000 - days_ago * 24 * 60 * 60);
        std::process::Command::new("git")
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Demo Developer")
            .env("GIT_AUTHOR_EMAIL", "demo@example.com")
            .env("GIT_COMMITTER_NAME", "Demo Developer")
            .env("GIT_COMMITTER_EMAIL", "demo@example.com")
            .env("GIT_AUTHOR_DATE", &at)
            .env("GIT_COMMITTER_DATE", &at)
            .output()
            .is_ok_and(|o| o.status.success())
    };
    if !git(&["init", "-q", "-b", "main"], 0) || !git(&["add", "-A"], 0) {
        return;
    }
    for (n, (subject, days_ago)) in history.iter().enumerate() {
        let empty = if n == 0 { None } else { Some("--allow-empty") };
        let args: Vec<&str> = ["commit", "-q", "-m", subject]
            .into_iter()
            .chain(empty)
            .collect();
        git(&args, *days_ago);
    }
}

/// Writes the demo folders into `<config_dir>/demo-projects/`, replacing what was there, and returns the
/// settings and registry that list them.
pub fn write(config_dir: &Path, now_ms: i64) -> AppResult<Seed> {
    let root = config_dir.join(DEMO_FOLDER);
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    create_private_dir(&root)?;
    let mut known = Vec::new();
    for (n, demo) in projects().into_iter().enumerate() {
        let dir = root.join(demo.slug);
        fs::create_dir_all(&dir)?;
        if let Some(Value::Object(object)) = demo.file {
            file::write(&dir, &object)?;
        }
        if let Some((name, manifest)) = demo.manifest {
            fs::write(dir.join(name), serde_json::to_vec_pretty(&manifest)?)?;
        }
        for (n, (path, text)) in demo.files.iter().enumerate() {
            let file = dir.join(path);
            if let Some(folder) = file.parent() {
                fs::create_dir_all(folder)?;
            }
            fs::write(&file, text)?;
            // Edited one after another over the last days, the last file first.
            let edited =
                UNIX_EPOCH + Duration::from_millis((now_ms - (n as i64 + 1) * 7 * HOUR) as u64);
            fs::File::options()
                .write(true)
                .open(&file)?
                .set_modified(edited)?;
        }
        if !demo.history.is_empty() {
            commit_history(&dir, demo.history, now_ms);
        }
        known.push(KnownProject {
            id: format!("demo{:08}", n + 1),
            path: dir.to_string_lossy().into_owned(),
            added_at: now_ms - demo.added_days_ago * DAY,
            last_opened_at: demo.opened_minutes_ago.map(|m| now_ms - m * MINUTE),
        });
    }
    Ok(Seed {
        settings: Settings {
            tags: ["Client", "Personal", "Experiments", "Ideas"]
                .map(String::from)
                .to_vec(),
            default_editor: "Visual Studio Code".into(),
            git_client: "GitHub Desktop".into(),
            ..Settings::default()
        },
        registry: Registry { projects: known },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::projects::FileRead;
    use crate::test_support::temp_config_dir;
    use std::collections::HashSet;

    const NOW: i64 = 1_800_000_000_000;

    fn slug(project: &KnownProject) -> String {
        Path::new(&project.path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn the_seed_registers_seven_projects_with_unique_ids_in_the_demo_folder() {
        let config = temp_config_dir();
        let seed = write(config.path(), NOW).unwrap();

        let projects = &seed.registry.projects;
        let slugs: Vec<String> = projects.iter().map(slug).collect();
        assert_eq!(
            slugs,
            [
                "acme-website",
                "orbit-api",
                "pixel-notes",
                "garden-planner",
                "weather-widget",
                "old-portfolio",
                "scratchpad"
            ]
        );
        let ids: HashSet<&str> = projects.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids.len(), 7);
        assert_eq!(projects[0].id, "demo00000001");
        for project in projects {
            assert!(Path::new(&project.path).starts_with(config.path().join(DEMO_FOLDER)));
            assert!(Path::new(&project.path).is_dir(), "{}", project.path);
        }
    }

    #[test]
    fn every_demo_folder_has_its_project_file_except_the_scratchpad() {
        let config = temp_config_dir();
        let seed = write(config.path(), NOW).unwrap();

        for project in &seed.registry.projects {
            let read = file::read(Path::new(&project.path));
            match slug(project).as_str() {
                "scratchpad" => assert_eq!(read, FileRead::Missing),
                other => assert!(matches!(read, FileRead::Found(_)), "{other}"),
            }
        }
        let acme = config.path().join(DEMO_FOLDER).join("acme-website");
        assert!(acme.join("package.json").is_file());
        assert!(acme.join(file::FILE_NAME).is_file());
    }

    #[test]
    fn the_acme_website_is_a_ddev_project_with_a_mariadb_database() {
        let config = temp_config_dir();
        write(config.path(), NOW).unwrap();

        let ddev_config = config
            .path()
            .join(DEMO_FOLDER)
            .join("acme-website")
            .join(".ddev/config.yaml");

        assert_eq!(
            fs::read_to_string(ddev_config).unwrap(),
            "name: acme-website\ntype: php\ndatabase:\n  type: mariadb\n  version: \"10.11\"\n"
        );
    }

    #[test]
    fn the_seed_dates_are_relative_to_now() {
        let config = temp_config_dir();
        let seed = write(config.path(), NOW).unwrap();

        let acme = &seed.registry.projects[0];
        assert_eq!(acme.added_at, NOW - 120 * DAY);
        assert_eq!(acme.last_opened_at, Some(NOW - 12 * MINUTE));
        let scratchpad = &seed.registry.projects[6];
        assert_eq!(scratchpad.added_at, NOW - 3 * DAY);
        assert_eq!(scratchpad.last_opened_at, None);
    }

    #[test]
    fn the_seed_lists_the_tags_and_every_project_tag_is_among_them() {
        let config = temp_config_dir();
        let seed = write(config.path(), NOW).unwrap();

        assert_eq!(
            seed.settings.tags,
            ["Client", "Personal", "Experiments", "Ideas"]
        );
        for project in &seed.registry.projects {
            if let FileRead::Found(object) = file::read(Path::new(&project.path))
                && let Some(tag) = object
                    .get("meta")
                    .and_then(|m| m.get("tag"))
                    .and_then(Value::as_str)
            {
                assert!(seed.settings.tags.iter().any(|t| t == tag), "{tag}");
            }
        }
    }

    #[test]
    fn writing_the_seed_again_replaces_the_demo_folder() {
        let config = temp_config_dir();
        let root = config.path().join(DEMO_FOLDER);
        write(config.path(), NOW).unwrap();
        fs::write(root.join("stale.txt"), "left over").unwrap();
        fs::write(root.join("acme-website").join(file::FILE_NAME), "{}").unwrap();
        fs::write(root.join("scratchpad").join(file::FILE_NAME), "{}").unwrap();

        write(config.path(), NOW).unwrap();

        assert!(!root.join("stale.txt").exists());
        assert!(!root.join("scratchpad").join(file::FILE_NAME).exists());
        let FileRead::Found(acme) = file::read(&root.join("acme-website")) else {
            panic!("the acme project file is back");
        };
        assert_eq!(acme.get("name"), Some(&json!("Acme Website")));
    }

    fn git_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn commit_count(dir: &Path) -> String {
        let output = std::process::Command::new("git")
            .args(["rev-list", "--count", "HEAD"])
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn the_acme_website_and_orbit_api_are_git_repositories_with_their_history() {
        if !git_available() {
            return;
        }
        let config = temp_config_dir();
        write(config.path(), NOW).unwrap();
        let root = config.path().join(DEMO_FOLDER);

        for (slug, commits) in [("acme-website", "12"), ("orbit-api", "5")] {
            assert!(root.join(slug).join(".git").exists(), "{slug}");
            assert_eq!(commit_count(&root.join(slug)), commits, "{slug}");
        }
    }

    #[test]
    fn the_projects_without_a_history_are_not_repositories() {
        let config = temp_config_dir();
        write(config.path(), NOW).unwrap();
        let root = config.path().join(DEMO_FOLDER);

        for slug in [
            "pixel-notes",
            "garden-planner",
            "weather-widget",
            "scratchpad",
        ] {
            assert!(!root.join(slug).join(".git").exists(), "{slug}");
        }
    }

    #[test]
    fn the_acme_website_has_sources_to_count() {
        let config = temp_config_dir();
        write(config.path(), NOW).unwrap();
        let acme = config.path().join(DEMO_FOLDER).join("acme-website");

        assert!(
            fs::read_to_string(acme.join("src/main.js"))
                .unwrap()
                .starts_with("import { header }")
        );
        assert!(acme.join("src/components/pricing.js").is_file());
        assert!(acme.join("web/theme/functions.php").is_file());
    }

    #[test]
    fn the_seed_shows_as_seven_projects_with_the_scratchpad_unavailable() {
        let config = temp_config_dir();
        let seed = write(config.path(), NOW).unwrap();
        let data = Data::open_with_home(config.path(), None);
        data.reset(seed.settings, seed.registry).unwrap();

        let shown: Vec<(String, bool)> = data
            .projects()
            .into_iter()
            .map(|p| (p.name, p.available))
            .collect();

        assert_eq!(
            shown,
            [
                ("Acme Website".to_string(), true),
                ("Orbit API".to_string(), true),
                ("Pixel Notes".to_string(), true),
                ("Garden Planner".to_string(), true),
                ("weather-widget".to_string(), true),
                ("Old Portfolio".to_string(), true),
                ("scratchpad".to_string(), false),
            ]
        );
    }
}
