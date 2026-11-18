//! Opens a project's Claude Code environment in Claude Manager through its CLI, `claude-manager open <folder>
//! --profile <dir>` (ADR 0049). The command goes through the `System` port, so a debug build records it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::run::shell::quote;
use crate::run::system::Job;
use crate::state::AppState;

pub const PROGRAM: &str = "claude-manager";

/// Where `claude-manager` is looked for: its install folder in the home folder, then Homebrew's.
pub fn search_folders(home: Option<&Path>) -> Vec<PathBuf> {
    home.map(|home| home.join(".local/bin"))
        .into_iter()
        .chain(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from))
        .collect()
}

/// The first executable `claude-manager` in `folders`.
pub fn find(folders: &[PathBuf]) -> Option<PathBuf> {
    folders
        .iter()
        .map(|folder| folder.join(PROGRAM))
        .find(|path| {
            path.metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// The command line that opens `folder` in Claude Manager with `profile` as its profile. The profile always goes
/// along: without one, Claude Manager would keep whichever profile it has open.
fn open_job(program: &Path, folder: &Path, profile: &str) -> Job {
    Job {
        command: format!(
            "{} open {} --profile {}",
            quote(&program.to_string_lossy()),
            quote(&folder.to_string_lossy()),
            quote(profile)
        ),
        cwd: Some(folder.to_path_buf()),
        env: Vec::new(),
    }
}

/// The job for the project, refused while it has no `.claude` folder or Claude Manager isn't installed.
fn job_for(state: &AppState, id: &str) -> AppResult<Job> {
    let folder = PathBuf::from(state.data.project_dir(id)?);
    let claude = state
        .data
        .project_config(id)?
        .claude
        .ok_or_else(|| AppError::InvalidInput("The project has no .claude folder".into()))?;
    let program = state
        .data
        .claude_manager()
        .ok_or_else(|| AppError::Launch(format!("{PROGRAM} isn’t installed")))?;
    Ok(open_job(&program, &folder, &claude.profile))
}

#[tauri::command]
pub async fn open_claude_manager(app: AppHandle, id: String) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let job = job_for(&state, &id)?;
        state.system.spawn(&job)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn install(folder: &Path, mode: u32) -> PathBuf {
        fs::create_dir_all(folder).unwrap();
        let program = folder.join(PROGRAM);
        fs::write(&program, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(mode)).unwrap();
        program
    }

    #[test]
    fn the_search_folders_are_the_home_install_folder_then_homebrew() {
        assert_eq!(
            search_folders(Some(Path::new("/Users/demo"))),
            [
                PathBuf::from("/Users/demo/.local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
            ]
        );
    }

    #[test]
    fn without_a_home_folder_only_the_homebrew_folders_are_searched() {
        assert_eq!(
            search_folders(None),
            [
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin")
            ]
        );
    }

    #[test]
    fn find_returns_the_executable_in_the_first_folder_that_has_it() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let second = root.path().join("second");
        fs::create_dir_all(&first).unwrap();
        let in_second = install(&second, 0o755);
        assert_eq!(
            find(&[first.clone(), second.clone()]),
            Some(in_second.clone())
        );

        let in_first = install(&first, 0o755);
        assert_eq!(find(&[first, second]), Some(in_first));
    }

    #[test]
    fn find_skips_a_file_that_is_not_executable() {
        let root = tempfile::tempdir().unwrap();
        let plain = root.path().join("plain");
        let good = root.path().join("good");
        install(&plain, 0o644);
        let executable = install(&good, 0o755);

        assert_eq!(find(&[plain.clone()]), None);
        assert_eq!(find(&[plain, good]), Some(executable));
    }

    #[test]
    fn find_skips_a_directory_named_like_the_program() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(PROGRAM)).unwrap();

        assert_eq!(find(&[root.path().to_path_buf()]), None);
    }

    #[test]
    fn find_is_none_when_the_program_is_in_no_folder() {
        let root = tempfile::tempdir().unwrap();

        assert_eq!(find(&[root.path().to_path_buf()]), None);
        assert_eq!(find(&[root.path().join("missing")]), None);
        assert_eq!(find(&[]), None);
    }

    #[test]
    fn the_open_job_opens_the_folder_with_the_profile_in_the_folder() {
        let job = open_job(
            Path::new("/Users/demo/.local/bin/claude-manager"),
            Path::new("/work/orbit-api"),
            "/Users/demo/.claude-demo",
        );

        assert_eq!(
            job,
            Job {
                command: "'/Users/demo/.local/bin/claude-manager' open '/work/orbit-api' \
                          --profile '/Users/demo/.claude-demo'"
                    .into(),
                cwd: Some(PathBuf::from("/work/orbit-api")),
                env: Vec::new(),
            }
        );
    }

    #[test]
    fn the_open_job_quotes_paths_with_spaces_and_quotes() {
        let job = open_job(
            Path::new("/opt/my tools/claude-manager"),
            Path::new("/work/it's here"),
            "/profiles/o'brien team",
        );

        assert_eq!(
            job.command,
            r"'/opt/my tools/claude-manager' open '/work/it'\''s here' --profile '/profiles/o'\''brien team'"
        );
        assert_eq!(job.cwd, Some(PathBuf::from("/work/it's here")));
    }
}
