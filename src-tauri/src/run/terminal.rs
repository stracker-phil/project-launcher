//! Terminal sessions through `.command` files, so no Automation permission is needed to start one. A command
//! that is still running in its tab is brought to the front instead of started twice.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::shell::{file_safe, terminal_script};
use super::system::System;
use crate::error::AppResult;

/// Scripts older than this are deleted when the next one is written.
const KEEP_SCRIPTS: Duration = Duration::from_secs(60 * 60);
const DEFAULT_TERMINAL: &str = "Terminal";

/// What a launch did. `NeedsPermission`: Terminal or iTerm didn't select the running command's tab (most often
/// Automation is refused), so only the app came to the front; `Activated`: this terminal can't be asked for a
/// tab.
#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub enum Launched {
    Launched,
    Focused,
    NeedsPermission,
    Activated,
}

/// Where a running command records itself: its shell's pid and tab.
#[derive(Deserialize, Debug, PartialEq)]
struct Session {
    pid: u32,
    tty: String,
}

/// The session files of terminal commands that `Terminal::run` started.
pub struct Sessions {
    folder: PathBuf,
}

impl Sessions {
    /// The sessions of the terminal scripts in `folder`.
    pub fn in_folder(folder: &Path) -> Self {
        Sessions {
            folder: folder.join("sessions"),
        }
    }

    fn file(&self, key: &str, command: &str) -> PathBuf {
        self.folder
            .join(format!("{:016x}.json", fnv1a(&format!("{key}\0{command}"))))
    }

    /// The recorded session while its pid still runs in the same tab (closed windows leave stale files).
    fn running(&self, system: &dyn System, file: &Path) -> Option<Session> {
        let session: Session = serde_json::from_slice(&fs::read(file).ok()?).ok()?;
        let tty = system.tool("ps", &["-o", "tty=", "-p", &session.pid.to_string()])?;
        let tty = tty.trim();
        (!tty.is_empty() && session.tty.ends_with(tty)).then_some(session)
    }

    /// Whether `command`, started by `run` with this `key`, still runs in its tab.
    pub fn is_running(&self, system: &dyn System, key: &str, command: &str) -> bool {
        self.running(system, &self.file(key, command)).is_some()
    }
}

/// Opens terminals with the settings' terminal app.
pub struct Terminal<'a> {
    pub system: &'a dyn System,
    /// Holds the `.command` files and the `sessions/` folder.
    pub folder: PathBuf,
    /// The settings' terminal app; empty is Terminal.
    pub app: &'a str,
}

impl Terminal<'_> {
    fn app(&self) -> &str {
        let app = self.app.trim();
        if app.is_empty() {
            DEFAULT_TERMINAL
        } else {
            app
        }
    }

    fn sessions(&self) -> Sessions {
        Sessions::in_folder(&self.folder)
    }

    /// Opens a terminal in `dir` with `env`, running `command` when it has text.
    pub fn open(
        &self,
        name: &str,
        dir: &Path,
        env: &[(String, String)],
        command: Option<&str>,
        session: Option<&Path>,
    ) -> AppResult<()> {
        fs::create_dir_all(&self.folder)?;
        self.remove_old_scripts();
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let file = self
            .folder
            .join(format!("{}-{stamp}.command", file_safe(name)));
        fs::write(&file, terminal_script(dir, env, command, session))?;
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755))?;
        self.system
            .open(&["-a", self.app(), &file.to_string_lossy()])
    }

    /// Runs `command` in a new terminal, or brings its tab to the front while it still runs there.
    pub fn run(
        &self,
        key: &str,
        name: &str,
        dir: &Path,
        env: &[(String, String)],
        command: &str,
    ) -> AppResult<Launched> {
        let sessions = self.sessions();
        let file = sessions.file(key, command);
        if let Some(session) = sessions.running(self.system, &file) {
            return self.focus(&session);
        }
        fs::create_dir_all(&sessions.folder)?;
        self.open(name, dir, env, Some(command), Some(&file))?;
        Ok(Launched::Launched)
    }

    /// Whether `command`, started by `run` with this `key`, still runs in its tab.
    pub fn is_running(&self, key: &str, command: &str) -> bool {
        self.sessions().is_running(self.system, key, command)
    }

    fn focus(&self, session: &Session) -> AppResult<Launched> {
        let script = focus_script(self.app(), &session.tty);
        if let Some(script) = &script
            && self
                .system
                .tool("osascript", &["-e", script])
                .as_deref()
                .map(str::trim)
                == Some("focused")
        {
            return Ok(Launched::Focused);
        }
        self.system.open(&["-a", self.app()])?;
        Ok(if script.is_some() {
            Launched::NeedsPermission
        } else {
            Launched::Activated
        })
    }

    fn remove_old_scripts(&self) {
        let Ok(entries) = fs::read_dir(&self.folder) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > KEEP_SCRIPTS);
            if old && path.extension().is_some_and(|e| e == "command") {
                let _ = fs::remove_file(path);
            }
        }
    }
}

/// The AppleScript that selects the tab running on `tty`, for terminals that can be asked; it prints
/// "focused" when it found the tab.
fn focus_script(app: &str, tty: &str) -> Option<String> {
    let is_tty = tty
        .strip_prefix("/dev/ttys")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    if !is_tty {
        return None;
    }
    match app.to_lowercase().as_str() {
        "terminal" => Some(format!(
            r#"tell application "Terminal"
  repeat with w in windows
    repeat with t in tabs of w
      if tty of t is "{tty}" then
        set selected of t to true
        set index of w to 1
        activate
        return "focused"
      end if
    end repeat
  end repeat
end tell"#
        )),
        "iterm" | "iterm2" => Some(format!(
            r#"tell application "iTerm"
  repeat with w in windows
    repeat with t in tabs of w
      repeat with s in sessions of t
        if tty of s is "{tty}" then
          select w
          select t
          select s
          activate
          return "focused"
        end if
      end repeat
    end repeat
  end repeat
end tell"#
        )),
        _ => None,
    }
}

/// A hash that stays the same across runs and Rust versions, for session file names.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::system::Recorder;

    const COMMAND: &str = "npm run dev";
    const TTY: &str = "/dev/ttys004";
    const PS: &str = "ps -o tty= -p 4242";

    struct Fx {
        _tmp: tempfile::TempDir,
        folder: PathBuf,
        system: Recorder,
    }

    impl Fx {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let folder = tmp.path().join("terminal");
            Self {
                _tmp: tmp,
                folder,
                system: Recorder::default(),
            }
        }

        fn terminal<'a>(&'a self, app: &'a str) -> Terminal<'a> {
            Terminal {
                system: &self.system,
                folder: self.folder.clone(),
                app,
            }
        }

        fn scripts(&self) -> Vec<PathBuf> {
            let mut files: Vec<PathBuf> = fs::read_dir(&self.folder)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|e| e == "command"))
                        .collect()
                })
                .unwrap_or_default();
            files.sort();
            files
        }

        /// Starts `COMMAND` through `run` and records the session the script would write: pid 4242 on `TTY`.
        fn start_session(&self, app: &str) {
            let launched = self
                .terminal(app)
                .run("demo", "Demo", Path::new("/work/demo"), &[], COMMAND)
                .unwrap();
            assert_eq!(launched, Launched::Launched);
            let script = fs::read_to_string(self.scripts().last().unwrap()).unwrap();
            let line = script.lines().find(|l| l.starts_with("printf")).unwrap();
            let path = line.rsplit_once("> ").unwrap().1.trim_matches('\'');
            fs::write(path, format!(r#"{{"pid":4242,"tty":"{TTY}"}}"#)).unwrap();
        }

        fn run_again(&self, app: &str) -> Launched {
            self.terminal(app)
                .run("demo", "Demo", Path::new("/work/demo"), &[], COMMAND)
                .unwrap()
        }
    }

    #[test]
    fn open_writes_an_executable_script_and_opens_it_with_the_terminal_app() {
        let fx = Fx::new();
        let env = vec![("STAGE".to_string(), "demo".to_string())];

        fx.terminal("Ghostty")
            .open(
                "Demo App",
                Path::new("/work/demo"),
                &env,
                Some(COMMAND),
                None,
            )
            .unwrap();

        let files = fx.scripts();
        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert!(
            file.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("Demo-App-")
        );
        assert_eq!(
            fs::read_to_string(file).unwrap(),
            terminal_script(Path::new("/work/demo"), &env, Some(COMMAND), None)
        );
        assert_eq!(
            fs::metadata(file).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fx.system.calls(),
            [format!("open: -a Ghostty {}", file.display())]
        );
    }

    #[test]
    fn an_empty_terminal_app_means_terminal() {
        for app in ["", "   "] {
            let fx = Fx::new();

            fx.terminal(app)
                .open("Demo", Path::new("/work/demo"), &[], None, None)
                .unwrap();

            assert_eq!(
                fx.system.calls(),
                [format!("open: -a Terminal {}", fx.scripts()[0].display())],
                "{app:?}"
            );
        }
    }

    #[test]
    fn opening_removes_old_scripts_and_keeps_other_files() {
        let fx = Fx::new();
        fs::create_dir_all(&fx.folder).unwrap();
        let old = fx.folder.join("old-1.command");
        let note = fx.folder.join("notes.txt");
        for path in [&old, &note] {
            fs::write(path, "x").unwrap();
            fs::File::options()
                .write(true)
                .open(path)
                .unwrap()
                .set_modified(SystemTime::now() - Duration::from_secs(2 * 60 * 60))
                .unwrap();
        }

        fx.terminal("Terminal")
            .open("Demo", Path::new("/work/demo"), &[], None, None)
            .unwrap();

        assert!(!old.exists());
        assert!(note.exists());
        assert_eq!(fx.scripts().len(), 1);
    }

    #[test]
    fn run_without_a_session_launches_a_script_that_records_its_session() {
        let fx = Fx::new();

        let launched = fx
            .terminal("Terminal")
            .run("demo", "Demo", Path::new("/work/demo"), &[], COMMAND)
            .unwrap();

        assert_eq!(launched, Launched::Launched);
        let script = fs::read_to_string(&fx.scripts()[0]).unwrap();
        assert!(script.contains(COMMAND));
        assert!(script.lines().any(|l| l.starts_with("printf")));
        assert!(fx.system.calls()[0].starts_with("open: -a Terminal "));
    }

    #[test]
    fn a_running_command_in_terminal_or_iterm_is_focused_when_osascript_finds_its_tab() {
        for app in ["Terminal", "iTerm2"] {
            let fx = Fx::new();
            fx.start_session(app);
            fx.system.reply(PS, "ttys004\n");
            assert_eq!(fx.run_again(app), Launched::NeedsPermission, "{app}");
            let script_call = fx
                .system
                .calls()
                .into_iter()
                .find(|c| c.starts_with("tool: osascript -e "))
                .unwrap();
            fx.system
                .reply(script_call.strip_prefix("tool: ").unwrap(), "focused\n");

            assert_eq!(fx.run_again(app), Launched::Focused, "{app}");
            assert_eq!(fx.scripts().len(), 1, "{app}");
        }
    }

    #[test]
    fn a_running_command_without_osascript_permission_only_brings_the_app_forward() {
        let fx = Fx::new();
        fx.start_session("Terminal");
        fx.system.reply(PS, "ttys004");

        assert_eq!(fx.run_again("Terminal"), Launched::NeedsPermission);

        assert_eq!(fx.system.calls().last().unwrap(), "open: -a Terminal");
        assert_eq!(fx.scripts().len(), 1);
    }

    #[test]
    fn a_running_command_in_another_terminal_app_is_activated() {
        let fx = Fx::new();
        fx.start_session("Ghostty");
        fx.system.reply(PS, "ttys004");

        assert_eq!(fx.run_again("Ghostty"), Launched::Activated);

        assert_eq!(fx.system.calls().last().unwrap(), "open: -a Ghostty");
        assert_eq!(fx.scripts().len(), 1);
    }

    #[test]
    fn a_stale_session_launches_the_command_anew() {
        let cases = [
            ("ps finds nothing", None),
            ("ps prints nothing", Some("\n")),
            ("the pid now runs in another tab", Some("ttys009\n")),
        ];
        for (name, ps) in cases {
            let fx = Fx::new();
            fx.start_session("Terminal");
            if let Some(ps) = ps {
                fx.system.reply(PS, ps);
            }

            assert_eq!(fx.run_again("Terminal"), Launched::Launched, "{name}");
            assert!(
                !fx.system.calls().iter().any(|c| c.contains("osascript")),
                "{name}"
            );
        }
    }

    #[test]
    fn is_running_follows_the_recorded_session_and_its_tab() {
        let fx = Fx::new();
        let terminal = fx.terminal("Terminal");
        assert!(!terminal.is_running("demo", COMMAND));

        fx.start_session("Terminal");
        assert!(!terminal.is_running("demo", COMMAND));

        fx.system.reply(PS, "ttys004\n");
        assert!(terminal.is_running("demo", COMMAND));
        assert!(!terminal.is_running("other", COMMAND));
        assert!(!terminal.is_running("demo", "npm test"));
    }

    #[test]
    fn sessions_of_a_folder_tell_whether_a_command_still_runs_without_a_terminal_app() {
        let fx = Fx::new();
        let sessions = Sessions::in_folder(&fx.folder);
        assert!(!sessions.is_running(&fx.system, "demo", COMMAND));

        fx.start_session("Terminal");
        fx.system.reply(PS, "ttys004\n");

        assert!(sessions.is_running(&fx.system, "demo", COMMAND));
        assert!(!sessions.is_running(&fx.system, "other", COMMAND));
    }

    #[test]
    fn a_damaged_session_file_counts_as_not_running() {
        let fx = Fx::new();
        fx.start_session("Terminal");
        fx.system.reply(PS, "ttys004");
        let sessions = fx.folder.join("sessions");
        for entry in fs::read_dir(&sessions).unwrap().flatten() {
            fs::write(entry.path(), "garbage").unwrap();
        }

        assert!(!fx.terminal("Terminal").is_running("demo", COMMAND));
    }

    #[test]
    fn launched_serializes_camel_case() {
        let cases = [
            (Launched::Launched, "launched"),
            (Launched::Focused, "focused"),
            (Launched::NeedsPermission, "needsPermission"),
            (Launched::Activated, "activated"),
        ];
        for (launched, expected) in cases {
            assert_eq!(
                serde_json::to_value(launched).unwrap(),
                serde_json::json!(expected)
            );
        }
    }
}
