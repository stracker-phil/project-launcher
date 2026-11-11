//! What kind of project a folder is and what it needs, from `package.json`, `composer.json`, `.nvmrc` and a
//! WordPress plugin or theme header; and the scripts those manifests offer.

use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

/// The first bytes of a file hold a WordPress header.
const HEADER_BYTES: u64 = 8192;
const NODE_LOCK_FILES: [(&str, Manager); 4] = [
    ("bun.lock", Manager::Bun),
    ("bun.lockb", Manager::Bun),
    ("pnpm-lock.yaml", Manager::Pnpm),
    ("yarn.lock", Manager::Yarn),
];

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Manager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Composer,
}

impl Manager {
    fn tool(self) -> &'static str {
        match self {
            Manager::Npm => "npm",
            Manager::Pnpm => "pnpm",
            Manager::Yarn => "yarn",
            Manager::Bun => "bun",
            Manager::Composer => "composer",
        }
    }

    /// The shell command that runs one of its scripts.
    pub fn command(self, script: &str) -> String {
        match self {
            Manager::Composer => format!("composer run-script {script}"),
            node => format!("{} run {script}", node.tool()),
        }
    }
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct PackageScript {
    pub manager: Manager,
    pub name: String,
    /// What the manifest says it runs.
    pub script: String,
    pub command: String,
}

/// What the manifests say.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Facts {
    /// "WordPress plugin", "Node package".
    pub kind: Option<String>,
    pub version: Option<String>,
    /// "PHP ≥ 7.4", "Node 20".
    pub requirements: Vec<String>,
    pub scripts: Vec<PackageScript>,
    /// The dependencies each manifest names, as "43 npm · 20 composer".
    pub packages: Option<String>,
}

fn read_json(file: &Path) -> Option<Map<String, Value>> {
    match serde_json::from_slice(&fs::read(file).ok()?).ok()? {
        Value::Object(object) => Some(object),
        _ => None,
    }
}

fn read_start(file: &Path) -> String {
    let mut text = String::new();
    if let Ok(handle) = fs::File::open(file) {
        let _ = handle.take(HEADER_BYTES).read_to_string(&mut text);
    }
    text
}

fn text(value: Option<&Value>) -> Option<String> {
    Some(value?.as_str()?.trim().to_string()).filter(|t| !t.is_empty())
}

fn field<'a>(object: Option<&'a Map<String, Value>>, key: &str) -> Option<&'a Map<String, Value>> {
    object?.get(key)?.as_object()
}

fn scripts_of(manifest: Option<&Map<String, Value>>, manager: Manager) -> Vec<PackageScript> {
    let Some(scripts) = field(manifest, "scripts") else {
        return Vec::new();
    };
    scripts
        .iter()
        // Composer's event hooks (post-install-cmd, …) aren't meant to be run by hand.
        .filter(|(name, _)| {
            manager != Manager::Composer || !(name.starts_with("pre-") || name.starts_with("post-"))
        })
        .map(|(name, script)| PackageScript {
            manager,
            name: name.clone(),
            script: match script {
                // Composer allows lists of commands.
                Value::Array(list) => list
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" && "),
                Value::String(script) => script.clone(),
                other => other.to_string(),
            },
            command: manager.command(name),
        })
        .collect()
}

fn dependencies<'a>(
    manifest: Option<&'a Map<String, Value>>,
    keys: [&str; 2],
) -> impl Iterator<Item = &'a String> {
    keys.into_iter()
        .filter_map(move |key| field(manifest, key))
        .flat_map(|deps| deps.keys())
}

/// "43 npm · 20 composer"; Composer's platform requirements (`php`, `ext-…`) aren't packages.
fn package_summary(
    package: Option<&Map<String, Value>>,
    composer: Option<&Map<String, Value>>,
    node: Manager,
) -> Option<String> {
    let node_count = dependencies(package, ["dependencies", "devDependencies"]).count();
    let composer_count = dependencies(composer, ["require", "require-dev"])
        .filter(|name| *name != "php" && !name.starts_with("ext-"))
        .count();
    let parts: Vec<String> = [(node_count, node), (composer_count, Manager::Composer)]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, manager)| format!("{count} {}", manager.tool()))
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn node_manager(dir: &Path) -> Manager {
    NODE_LOCK_FILES
        .iter()
        .find(|(file, _)| dir.join(file).is_file())
        .map_or(Manager::Npm, |(_, manager)| *manager)
}

/// `Name: value` in a WordPress header, after comment characters.
fn header_field(header: &str, name: &str) -> Option<String> {
    header.lines().find_map(|line| {
        let line = line.trim_start_matches(|c: char| " \t/*#@".contains(c));
        let value = line.strip_prefix(name)?.strip_prefix(':')?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

struct WordPress {
    kind: &'static str,
    version: Option<String>,
    requires_php: Option<String>,
    requires_wordpress: Option<String>,
}

fn wordpress_header(header: &str, name: &str, kind: &'static str) -> Option<WordPress> {
    header_field(header, name)?;
    Some(WordPress {
        kind,
        version: header_field(header, "Version"),
        requires_php: header_field(header, "Requires PHP"),
        requires_wordpress: header_field(header, "Requires at least"),
    })
}

/// A theme's `style.css`, else a root PHP file with a "Plugin Name:" header.
fn wordpress(dir: &Path) -> Option<WordPress> {
    if let Some(theme) = wordpress_header(
        &read_start(&dir.join("style.css")),
        "Theme Name",
        "WordPress theme",
    ) {
        return Some(theme);
    }
    let mut php: Vec<_> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "php") && p.is_file())
        .collect();
    php.sort();
    php.iter()
        .find_map(|file| wordpress_header(&read_start(file), "Plugin Name", "WordPress plugin"))
}

/// `>=7.4` → `≥ 7.4`, `^8.1` or `~8.1` → `8.1+`; anything else as written.
fn readable(constraint: Option<String>) -> Option<String> {
    let constraint = constraint?;
    let numeric = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.');
    if let Some(minimum) = constraint
        .strip_prefix(">=")
        .map(str::trim)
        .filter(|m| numeric(m))
    {
        return Some(format!("≥ {minimum}"));
    }
    if let Some(caret) = constraint
        .strip_prefix(['^', '~'])
        .map(str::trim)
        .filter(|m| numeric(m))
    {
        return Some(format!("{caret}+"));
    }
    Some(constraint)
}

fn composer_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "wordpress-plugin" => Some("WordPress plugin"),
        "wordpress-theme" => Some("WordPress theme"),
        "wordpress-muplugin" => Some("WordPress must-use plugin"),
        "library" => Some("Composer library"),
        "project" => Some("Composer project"),
        _ => None,
    }
}

pub fn facts(dir: &Path) -> Facts {
    let package = read_json(&dir.join("package.json"));
    let composer = read_json(&dir.join("composer.json"));
    let nvmrc = Some(
        read_start(&dir.join(".nvmrc"))
            .trim()
            .trim_start_matches('v')
            .to_string(),
    )
    .filter(|n| !n.is_empty());
    let wordpress = wordpress(dir);
    let kind = wordpress
        .as_ref()
        .map(|w| w.kind.to_string())
        .or_else(|| {
            text(composer.as_ref()?.get("type"))
                .and_then(|k| composer_kind(&k))
                .map(str::to_string)
        })
        .or_else(|| composer.as_ref().map(|_| "Composer package".to_string()))
        .or_else(|| package.as_ref().map(|_| "Node package".to_string()));
    // WordPress headers state a minimum, .nvmrc an exact version, manifests a constraint.
    let php = match wordpress.as_ref().and_then(|w| w.requires_php.clone()) {
        Some(minimum) => Some(format!("≥ {minimum}")),
        None => readable(text(
            field(composer.as_ref(), "require").and_then(|r| r.get("php")),
        )),
    };
    let node = nvmrc.or_else(|| {
        readable(text(
            field(package.as_ref(), "engines").and_then(|e| e.get("node")),
        ))
    });
    let requirements = [
        php.map(|p| format!("PHP {p}")),
        wordpress
            .as_ref()
            .and_then(|w| w.requires_wordpress.clone())
            .map(|w| format!("WordPress ≥ {w}")),
        node.map(|n| format!("Node {n}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let version = wordpress
        .as_ref()
        .and_then(|w| w.version.clone())
        .or_else(|| text(package.as_ref()?.get("version")))
        .or_else(|| text(composer.as_ref()?.get("version")));
    let node = node_manager(dir);
    let mut scripts = scripts_of(package.as_ref(), node);
    scripts.extend(scripts_of(composer.as_ref(), Manager::Composer));
    Facts {
        kind,
        version,
        requirements,
        scripts,
        packages: package_summary(package.as_ref(), composer.as_ref(), node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn folder(files: &[(&str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            fs::write(dir.path().join(name), text).unwrap();
        }
        dir
    }

    fn script(manager: Manager, name: &str, script: &str, command: &str) -> PackageScript {
        PackageScript {
            manager,
            name: name.into(),
            script: script.into(),
            command: command.into(),
        }
    }

    #[test]
    fn a_folder_without_manifests_has_no_kind_and_no_scripts() {
        let dir = folder(&[("notes.txt", "hello")]);

        assert_eq!(facts(dir.path()), Facts::default());
    }

    #[test]
    fn a_package_json_gives_the_name_kind_version_and_scripts() {
        let dir = folder(&[(
            "package.json",
            r#"{"name":"demo-app","version":"2.4.0","scripts":{"build":"vite build","dev":"vite"}}"#,
        )]);

        assert_eq!(
            facts(dir.path()),
            Facts {
                kind: Some("Node package".into()),
                version: Some("2.4.0".into()),
                requirements: vec![],
                scripts: vec![
                    script(Manager::Npm, "build", "vite build", "npm run build"),
                    script(Manager::Npm, "dev", "vite", "npm run dev"),
                ],
                packages: None,
            }
        );
    }

    #[test]
    fn the_node_requirement_is_written_readably() {
        for (engines, expected) in [
            (">=20", "Node ≥ 20"),
            ("^18", "Node 18+"),
            ("~18.2", "Node 18.2+"),
            ("20 || 22", "Node 20 || 22"),
        ] {
            let dir = folder(&[(
                "package.json",
                &json!({ "engines": { "node": engines } }).to_string(),
            )]);

            assert_eq!(facts(dir.path()).requirements, [expected], "{engines}");
        }
    }

    #[test]
    fn an_nvmrc_wins_over_the_engines_field() {
        let dir = folder(&[
            ("package.json", r#"{"engines":{"node":">=18"}}"#),
            (".nvmrc", "v20.11\n"),
        ]);

        assert_eq!(facts(dir.path()).requirements, ["Node 20.11"]);
    }

    #[test]
    fn the_lock_file_picks_the_node_manager_for_every_script() {
        for (lock, manager, command) in [
            ("bun.lock", Manager::Bun, "bun run dev"),
            ("bun.lockb", Manager::Bun, "bun run dev"),
            ("pnpm-lock.yaml", Manager::Pnpm, "pnpm run dev"),
            ("yarn.lock", Manager::Yarn, "yarn run dev"),
            ("package-lock.json", Manager::Npm, "npm run dev"),
        ] {
            let dir = folder(&[
                ("package.json", r#"{"scripts":{"dev":"vite"}}"#),
                (lock, ""),
            ]);

            assert_eq!(
                facts(dir.path()).scripts,
                [script(manager, "dev", "vite", command)],
                "{lock}"
            );
        }
    }

    #[test]
    fn a_composer_json_gives_the_kind_php_requirement_and_runnable_scripts() {
        let dir = folder(&[(
            "composer.json",
            &json!({
                "name": "orbit/api",
                "type": "project",
                "require": { "php": ">=8.2" },
                "scripts": {
                    "lint": "phpcs",
                    "post-install-cmd": "echo installed",
                    "pre-update-cmd": ["echo before", "echo again"],
                    "test": ["phpunit", "phpstan"],
                },
            })
            .to_string(),
        )]);

        assert_eq!(
            facts(dir.path()),
            Facts {
                kind: Some("Composer project".into()),
                version: None,
                requirements: vec!["PHP ≥ 8.2".into()],
                scripts: vec![
                    script(
                        Manager::Composer,
                        "lint",
                        "phpcs",
                        "composer run-script lint"
                    ),
                    script(
                        Manager::Composer,
                        "test",
                        "phpunit && phpstan",
                        "composer run-script test"
                    ),
                ],
                packages: None,
            }
        );
    }

    #[test]
    fn the_composer_type_maps_to_a_kind() {
        for (kind, expected) in [
            ("wordpress-plugin", "WordPress plugin"),
            ("wordpress-theme", "WordPress theme"),
            ("wordpress-muplugin", "WordPress must-use plugin"),
            ("library", "Composer library"),
            ("metapackage", "Composer package"),
        ] {
            let dir = folder(&[("composer.json", &json!({ "type": kind }).to_string())]);

            assert_eq!(facts(dir.path()).kind.as_deref(), Some(expected), "{kind}");
        }
    }

    #[test]
    fn a_composer_json_without_a_type_is_a_composer_package() {
        let dir = folder(&[("composer.json", "{}")]);

        assert_eq!(facts(dir.path()).kind.as_deref(), Some("Composer package"));
    }

    #[test]
    fn a_composer_package_wins_the_kind_over_a_node_package() {
        let dir = folder(&[("composer.json", "{}"), ("package.json", "{}")]);

        assert_eq!(facts(dir.path()).kind.as_deref(), Some("Composer package"));
    }

    #[test]
    fn a_plugin_header_in_a_root_php_file_gives_kind_version_and_requirements() {
        let dir = folder(&[
            ("index.php", "<?php\n// Silence is golden.\n"),
            (
                "demo-plugin.php",
                "<?php\n/**\n * Plugin Name: Demo Plugin\n * Version: 3.1.2\n * Requires at least: 6.0\n * Requires PHP: 7.4\n */\n",
            ),
        ]);

        assert_eq!(
            facts(dir.path()),
            Facts {
                kind: Some("WordPress plugin".into()),
                version: Some("3.1.2".into()),
                requirements: vec!["PHP ≥ 7.4".into(), "WordPress ≥ 6.0".into()],
                scripts: vec![],
                packages: None,
            }
        );
    }

    #[test]
    fn a_theme_stylesheet_gives_a_wordpress_theme() {
        let dir = folder(&[(
            "style.css",
            "/*\nTheme Name: Demo Theme\nVersion: 1.2.0\nRequires at least: 6.1\n*/\nbody {}\n",
        )]);

        assert_eq!(
            facts(dir.path()),
            Facts {
                kind: Some("WordPress theme".into()),
                version: Some("1.2.0".into()),
                requirements: vec!["WordPress ≥ 6.1".into()],
                scripts: vec![],
                packages: None,
            }
        );
    }

    #[test]
    fn a_stylesheet_without_a_theme_name_is_not_a_theme() {
        let dir = folder(&[("style.css", "body { margin: 0; }\n")]);

        assert_eq!(facts(dir.path()).kind, None);
    }

    #[test]
    fn the_wordpress_header_wins_over_composer() {
        let dir = folder(&[
            (
                "demo-plugin.php",
                "<?php\n/*\nPlugin Name: Demo Plugin\nVersion: 4.0.0\nRequires PHP: 7.4\n*/\n",
            ),
            (
                "composer.json",
                r#"{"type":"library","version":"1.0.0","require":{"php":"^8.1"}}"#,
            ),
        ]);

        let facts = facts(dir.path());

        assert_eq!(facts.kind.as_deref(), Some("WordPress plugin"));
        assert_eq!(facts.version.as_deref(), Some("4.0.0"));
        assert_eq!(facts.requirements, ["PHP ≥ 7.4"]);
    }

    #[test]
    fn a_manifest_that_is_not_a_json_object_is_ignored() {
        let dir = folder(&[("package.json", "[1, 2]"), ("composer.json", "not json")]);

        assert_eq!(facts(dir.path()), Facts::default());
    }

    #[test]
    fn node_dependencies_are_counted_under_the_manager_of_the_lock_file() {
        let package = json!({
            "dependencies": { "react": "^19", "zustand": "^5" },
            "devDependencies": { "vite": "^7" },
        })
        .to_string();
        for (lock, expected) in [
            ("package-lock.json", "3 npm"),
            ("pnpm-lock.yaml", "3 pnpm"),
            ("yarn.lock", "3 yarn"),
            ("bun.lock", "3 bun"),
            ("bun.lockb", "3 bun"),
        ] {
            let dir = folder(&[("package.json", &package), (lock, "")]);

            assert_eq!(
                facts(dir.path()).packages.as_deref(),
                Some(expected),
                "{lock}"
            );
        }
    }

    #[test]
    fn composer_packages_leave_out_the_php_version_and_extensions() {
        let dir = folder(&[(
            "composer.json",
            &json!({
                "require": { "php": ">=8.2", "ext-json": "*", "monolog/monolog": "^3" },
                "require-dev": { "phpunit/phpunit": "^11", "ext-xdebug": "*" },
            })
            .to_string(),
        )]);

        assert_eq!(facts(dir.path()).packages.as_deref(), Some("2 composer"));
    }

    #[test]
    fn packages_of_both_manifests_are_joined_node_first() {
        let dir = folder(&[
            (
                "package.json",
                r#"{"dependencies":{"react":"^19"},"devDependencies":{"vite":"^7"}}"#,
            ),
            (
                "composer.json",
                r#"{"require":{"php":">=8.2","monolog/monolog":"^3"}}"#,
            ),
            ("pnpm-lock.yaml", ""),
        ]);

        assert_eq!(
            facts(dir.path()).packages.as_deref(),
            Some("2 pnpm · 1 composer")
        );
    }

    #[test]
    fn a_manifest_without_dependencies_is_left_out_of_the_packages() {
        let dir = folder(&[
            (
                "package.json",
                r#"{"dependencies":{},"scripts":{"dev":"vite"}}"#,
            ),
            ("composer.json", r#"{"require":{"php":">=8.2"}}"#),
        ]);
        assert_eq!(facts(dir.path()).packages, None);

        let dir = folder(&[
            ("package.json", "{}"),
            ("composer.json", r#"{"require":{"monolog/monolog":"^3"}}"#),
        ]);
        assert_eq!(facts(dir.path()).packages.as_deref(), Some("1 composer"));
    }

    #[test]
    fn node_and_composer_scripts_are_listed_together_node_first() {
        let dir = folder(&[
            ("package.json", r#"{"scripts":{"dev":"vite"}}"#),
            ("composer.json", r#"{"scripts":{"test":"phpunit"}}"#),
            ("pnpm-lock.yaml", ""),
        ]);

        assert_eq!(
            facts(dir.path()).scripts,
            [
                script(Manager::Pnpm, "dev", "vite", "pnpm run dev"),
                script(
                    Manager::Composer,
                    "test",
                    "phpunit",
                    "composer run-script test"
                ),
            ]
        );
    }
}
