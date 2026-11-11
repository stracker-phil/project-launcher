//! Settings → Status: the facts about this build and where it keeps its files.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

/// Where `scripts/install.sh` links the `launcher` command.
const CLI_LINK: &str = ".local/bin/launcher";

/// Paths start with `~` for the home folder.
#[derive(Serialize, TS, Debug, Clone, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    /// `Launcher`, or `Launcher (Debug)` in debug builds.
    pub name: String,
    pub version: String,
    pub debug: bool,
    pub data_folder: String,
    pub log_folder: String,
    pub command_line: CommandLine,
}

/// The `launcher` command on the PATH, at `path`.
#[derive(Serialize, TS, Debug, Clone, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct CommandLine {
    pub path: String,
    pub link: CliLink,
}

#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum CliLink {
    /// Links to this app's binary.
    Installed,
    /// Missing, or links to something else.
    NotInstalled,
    /// Debug builds are never installed (ADR 0022).
    NotApplicable,
}

/// Whether `link` resolves to `exe`.
pub fn cli_link(link: &Path, exe: &Path, debug: bool) -> CliLink {
    if debug {
        return CliLink::NotApplicable;
    }
    match (link.canonicalize(), exe.canonicalize()) {
        (Ok(target), Ok(resolved)) if target == resolved => CliLink::Installed,
        _ => CliLink::NotInstalled,
    }
}

/// `path` with a leading `home` shown as `~`.
pub fn tilde(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

fn in_folder_name(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// `home` found between `before` and `after` is the whole start of a path, not part of a longer name.
fn is_home_path(before: &str, after: &str) -> bool {
    let starts_path = before
        .chars()
        .next_back()
        .is_none_or(|c| !in_folder_name(c) && c != '/');
    starts_path && after.chars().next().is_none_or(|c| !in_folder_name(c))
}

/// `text` (a command) with every path that starts in `home` shown from `~`.
pub fn tilde_in(text: &str, home: Option<&Path>) -> String {
    let Some(home) = home.and_then(Path::to_str).filter(|h| !h.is_empty()) else {
        return text.to_string();
    };
    let mut shown = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(home) {
        let (before, after) = (&rest[..at], &rest[at + home.len()..]);
        shown.push_str(before);
        shown.push_str(if is_home_path(before, after) {
            "~"
        } else {
            home
        });
        rest = after;
    }
    shown.push_str(rest);
    shown
}

/// The status of this build, from paths already read; without a home folder the command line shows where it
/// would be linked and counts as not installed.
pub fn build_status(
    home: Option<&Path>,
    exe: &Path,
    debug: bool,
    data_dir: &Path,
    log_dir: &Path,
) -> AppStatus {
    let command_line = match home {
        Some(home) => {
            let link = home.join(CLI_LINK);
            CommandLine {
                path: tilde(&link, Some(home)),
                link: cli_link(&link, exe, debug),
            }
        }
        None => CommandLine {
            path: format!("~/{CLI_LINK}"),
            link: if debug {
                CliLink::NotApplicable
            } else {
                CliLink::NotInstalled
            },
        },
    };
    AppStatus {
        name: crate::APP_NAME.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        debug,
        data_folder: tilde(data_dir, home),
        log_folder: tilde(log_dir, home),
        command_line,
    }
}

fn io_error(e: tauri::Error) -> AppError {
    AppError::Io(e.to_string())
}

#[tauri::command]
pub fn app_status(app: AppHandle) -> AppResult<AppStatus> {
    let home = std::env::home_dir();
    let exe = std::env::current_exe()?;
    let data_dir = app.path().app_config_dir().map_err(io_error)?;
    let log_dir = app.path().app_log_dir().map_err(io_error)?;
    Ok(build_status(
        home.as_deref(),
        &exe,
        cfg!(debug_assertions),
        &data_dir,
        &log_dir,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn tilde_shortens_only_paths_inside_the_home_folder() {
        let home = Path::new("/Users/demo");
        for (path, home, expected) in [
            ("/Users/demo", Some(home), "~"),
            (
                "/Users/demo/.local/bin/launcher",
                Some(home),
                "~/.local/bin/launcher",
            ),
            ("/opt/tools/launcher", Some(home), "/opt/tools/launcher"),
            ("/Users/demo2/notes", Some(home), "/Users/demo2/notes"),
            ("/Users/demo/notes", None, "/Users/demo/notes"),
        ] {
            assert_eq!(tilde(Path::new(path), home), expected, "{path}");
        }
    }

    #[test]
    fn tilde_in_shows_every_path_inside_home_from_the_tilde() {
        let home = Path::new("/Users/demo");
        for (text, expected) in [
            (
                "ddev export-db --file=/Users/demo/x/b.sql",
                "ddev export-db --file=~/x/b.sql",
            ),
            ("/Users/demo", "~"),
            ("cd /Users/demo && ls", "cd ~ && ls"),
            ("cp /Users/demo/a /Users/demo/b", "cp ~/a ~/b"),
            ("ls /Users/demoX/a", "ls /Users/demoX/a"),
            ("cp /Users/demoX/a /Users/demo/b", "cp /Users/demoX/a ~/b"),
            ("ls /opt/tools", "ls /opt/tools"),
            ("ls foo/Users/demo/x", "ls foo/Users/demo/x"),
            ("", ""),
        ] {
            assert_eq!(tilde_in(text, Some(home)), expected, "{text}");
        }
    }

    #[test]
    fn tilde_in_leaves_text_unchanged_without_a_usable_home() {
        let text = "ls /Users/demo/notes";
        assert_eq!(tilde_in(text, None), text);
        assert_eq!(tilde_in(text, Some(Path::new(""))), text);
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        exe: PathBuf,
        other: PathBuf,
        dir: PathBuf,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let exe = dir.join("launcher-bin");
        let other = dir.join("other-bin");
        fs::write(&exe, "exe").unwrap();
        fs::write(&other, "other").unwrap();
        Fixture {
            _dir: tmp,
            exe,
            other,
            dir,
        }
    }

    #[test]
    fn a_debug_build_is_never_installed() {
        let f = fixture();
        let link = f.dir.join("link");
        symlink(&f.exe, &link).unwrap();
        assert_eq!(cli_link(&link, &f.exe, true), CliLink::NotApplicable);
        assert_eq!(
            cli_link(&f.dir.join("missing"), &f.exe, true),
            CliLink::NotApplicable
        );
    }

    #[test]
    fn a_link_to_the_executable_is_installed() {
        let f = fixture();
        let link = f.dir.join("link");
        symlink(&f.exe, &link).unwrap();
        assert_eq!(cli_link(&link, &f.exe, false), CliLink::Installed);
    }

    #[test]
    fn a_chain_of_links_ending_at_the_executable_is_installed() {
        let f = fixture();
        let middle = f.dir.join("middle");
        let link = f.dir.join("link");
        symlink(&f.exe, &middle).unwrap();
        symlink(&middle, &link).unwrap();
        assert_eq!(cli_link(&link, &f.exe, false), CliLink::Installed);
    }

    #[test]
    fn a_link_to_another_file_a_missing_link_and_a_dangling_link_are_not_installed() {
        let f = fixture();
        let elsewhere = f.dir.join("elsewhere");
        let dangling = f.dir.join("dangling");
        symlink(&f.other, &elsewhere).unwrap();
        symlink(f.dir.join("gone"), &dangling).unwrap();
        for link in [elsewhere, f.dir.join("missing"), dangling] {
            assert_eq!(
                cli_link(&link, &f.exe, false),
                CliLink::NotInstalled,
                "{link:?}"
            );
        }
    }

    #[test]
    fn the_status_shows_paths_under_home_with_a_tilde() {
        let f = fixture();
        let home = Path::new("/Users/demo");
        let status = build_status(
            Some(home),
            &f.exe,
            false,
            Path::new("/Users/demo/Library/Application Support/Launcher"),
            Path::new("/var/log/launcher"),
        );
        assert_eq!(status.name, crate::APP_NAME);
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
        assert!(!status.debug);
        assert_eq!(status.data_folder, "~/Library/Application Support/Launcher");
        assert_eq!(status.log_folder, "/var/log/launcher");
        assert_eq!(
            status.command_line,
            CommandLine {
                path: "~/.local/bin/launcher".into(),
                link: CliLink::NotInstalled,
            }
        );
    }

    #[test]
    fn the_status_finds_the_command_line_link_in_the_home_folder() {
        let f = fixture();
        let bin = f.dir.join(".local/bin");
        fs::create_dir_all(&bin).unwrap();
        symlink(&f.exe, bin.join("launcher")).unwrap();
        let status = build_status(Some(&f.dir), &f.exe, false, &f.dir, &f.dir);
        assert_eq!(status.command_line.link, CliLink::Installed);
        assert_eq!(status.command_line.path, "~/.local/bin/launcher");
        assert_eq!(status.data_folder, "~");
    }

    #[test]
    fn a_debug_status_has_no_applicable_command_line() {
        let f = fixture();
        for home in [Some(Path::new("/Users/demo")), None] {
            let status = build_status(home, &f.exe, true, &f.dir, &f.dir);
            assert!(status.debug, "{home:?}");
            assert_eq!(status.command_line.link, CliLink::NotApplicable, "{home:?}");
        }
    }

    #[test]
    fn without_a_home_folder_the_command_line_is_shown_unlinked_and_paths_stay_whole() {
        let f = fixture();
        let status = build_status(
            None,
            &f.exe,
            false,
            Path::new("/data/launcher"),
            Path::new("/logs/launcher"),
        );
        assert_eq!(status.data_folder, "/data/launcher");
        assert_eq!(status.log_folder, "/logs/launcher");
        assert_eq!(
            status.command_line,
            CommandLine {
                path: "~/.local/bin/launcher".into(),
                link: CliLink::NotInstalled,
            }
        );
    }

    #[test]
    fn the_status_serializes_in_camel_case() {
        let status = AppStatus {
            name: "Launcher".into(),
            version: "1.2.3".into(),
            debug: false,
            data_folder: "~/data".into(),
            log_folder: "~/logs".into(),
            command_line: CommandLine {
                path: "~/.local/bin/launcher".into(),
                link: CliLink::Installed,
            },
        };
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            serde_json::json!({
                "name": "Launcher",
                "version": "1.2.3",
                "debug": false,
                "dataFolder": "~/data",
                "logFolder": "~/logs",
                "commandLine": { "path": "~/.local/bin/launcher", "link": "installed" },
            })
        );
    }

    #[test]
    fn the_cli_link_serializes_as_a_camel_case_word() {
        for (link, expected) in [
            (CliLink::Installed, "installed"),
            (CliLink::NotInstalled, "notInstalled"),
            (CliLink::NotApplicable, "notApplicable"),
        ] {
            assert_eq!(
                serde_json::to_value(link).unwrap(),
                serde_json::json!(expected)
            );
        }
    }
}
