//! The port to the Mac: starting and running processes, `open`, the clipboard. `Os` is the real one; debug
//! builds use the `Recorder` unless `LAUNCHER_REAL_LAUNCH=1` (ADR 0022).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};
use crate::lock::MutexExt;

/// Homebrew and the system folders, ahead of the inherited PATH: an app started from Finder gets a short one.
const EXTRA_PATH: [&str; 6] = [
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];
/// Captured output stops growing here.
const MAX_OUTPUT: usize = 512 * 1024;
/// How long a started command may take to fail before it counts as launched.
const SPAWN_GRACE: Duration = Duration::from_millis(2500);
const TOOL_TIMEOUT: Duration = Duration::from_secs(5);
/// `ddev describe` asks Docker and can take a while.
const INSPECT_TIMEOUT: Duration = Duration::from_secs(15);

/// A command for a login `zsh`, in a folder, with extra env vars.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Job {
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

/// What a finished (or stopped) command printed. `output` holds stdout and stderr as they came.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Output {
    pub code: Option<i32>,
    pub timed_out: bool,
    pub stdout: String,
    pub output: String,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.code == Some(0) && !self.timed_out
    }
}

pub trait System: Send + Sync {
    /// Starts a (possibly long-lived) command; an early failure is an error, a running one is launched.
    fn spawn(&self, job: &Job) -> AppResult<()>;
    /// Runs a command to its end, stopping it after `timeout`.
    fn run(&self, job: &Job, timeout: Duration) -> Output;
    /// Runs `/usr/bin/open` with these arguments.
    fn open(&self, args: &[&str]) -> AppResult<()>;
    /// Runs a program without a shell (`ps`, `osascript`); its stdout when it succeeded.
    fn tool(&self, program: &str, args: &[&str]) -> Option<String>;
    /// Runs a program without a shell, in `cwd` when given; what it printed, whatever its exit code.
    fn inspect(&self, cwd: Option<&Path>, program: &str, args: &[&str]) -> Output;
    fn copy(&self, text: &str) -> AppResult<()>;
}

pub fn system() -> Arc<dyn System> {
    #[cfg(debug_assertions)]
    if std::env::var("LAUNCHER_REAL_LAUNCH").as_deref() != Ok("1") {
        return Arc::new(Recorder::default());
    }
    Arc::new(Os)
}

pub struct Os;

/// Runs a read-only program (`git`, `du`) for real in every build, in `cwd` when given; what it printed,
/// whatever its exit code.
pub fn read(cwd: Option<&Path>, name: &str, args: &[&str]) -> Output {
    let mut command = program(name, args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    capture(command, INSPECT_TIMEOUT)
}

/// Homebrew's and the system's folders ahead of the inherited PATH.
fn path() -> String {
    let inherited = std::env::var("PATH").unwrap_or_default();
    EXTRA_PATH
        .iter()
        .copied()
        .chain(Some(inherited.as_str()).filter(|p| !p.is_empty()))
        .collect::<Vec<_>>()
        .join(":")
}

fn program(name: &str, args: &[&str]) -> Command {
    let mut command = Command::new(name);
    command.args(args).env("PATH", path());
    command
}

fn login_shell(job: &Job) -> Command {
    let mut command = Command::new("/bin/zsh");
    command.args(["-l", "-c", &job.command]);
    if let Some(cwd) = &job.cwd {
        command.current_dir(cwd);
    }
    command.env("PATH", path());
    command.envs(job.env.iter().map(|(k, v)| (k, v)));
    command
}

/// Starts `command` in its own process group, so stopping it stops what it started.
fn start(mut command: Command, stdout: Stdio) -> std::io::Result<Child> {
    use std::os::unix::process::CommandExt;
    command
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
}

fn stop_group(child: &Child) {
    // SAFETY: killpg only sends a signal; the group id is the child's own pid (process_group(0)).
    unsafe {
        libc::killpg(child.id() as libc::pid_t, libc::SIGTERM);
    }
}

/// Appends what `source` prints to `own` and `both`, each up to `MAX_OUTPUT`.
fn collect(
    mut source: impl Read + Send + 'static,
    own: Option<Arc<Mutex<String>>>,
    both: Arc<Mutex<String>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        while let Ok(n) = source.read(&mut buffer) {
            if n == 0 {
                break;
            }
            let text = String::from_utf8_lossy(&buffer[..n]);
            for target in own.iter().chain(Some(&both)) {
                let mut target = target.lock_unpoisoned();
                if target.len() < MAX_OUTPUT {
                    target.push_str(&text);
                }
            }
        }
    })
}

/// The child's exit status once it ended before `deadline`; `None` while it still runs then.
fn exit_by(
    child: &mut Child,
    deadline: Instant,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(Some(status)),
            None if Instant::now() >= deadline => return Ok(None),
            None => thread::sleep(Duration::from_millis(25)),
        }
    }
}

/// Waits for `child` until `timeout`; returns its exit code and whether the timeout stopped it.
fn wait_until(child: &mut Child, timeout: Duration) -> (Option<i32>, bool) {
    match exit_by(child, Instant::now() + timeout) {
        Ok(Some(status)) => (status.code(), false),
        Ok(None) => {
            stop_group(child);
            (child.wait().ok().and_then(|s| s.code()), true)
        }
        Err(_) => (None, false),
    }
}

fn capture(command: Command, timeout: Duration) -> Output {
    let mut child = match start(command, Stdio::piped()) {
        Ok(child) => child,
        Err(error) => {
            return Output {
                output: error.to_string(),
                ..Output::default()
            };
        }
    };
    let stdout = Arc::new(Mutex::new(String::new()));
    let both = Arc::new(Mutex::new(String::new()));
    let readers = [
        child
            .stdout
            .take()
            .map(|out| collect(out, Some(stdout.clone()), both.clone())),
        child
            .stderr
            .take()
            .map(|err| collect(err, None, both.clone())),
    ];
    let (code, timed_out) = wait_until(&mut child, timeout);
    for reader in readers.into_iter().flatten() {
        let _ = reader.join();
    }
    let take = |text: &Arc<Mutex<String>>| std::mem::take(&mut *text.lock_unpoisoned());
    Output {
        code,
        timed_out,
        stdout: take(&stdout),
        output: take(&both),
    }
}

fn failure(stderr: &str, code: Option<i32>) -> AppError {
    let message = stderr.trim();
    AppError::Launch(if message.is_empty() {
        match code {
            Some(code) => format!("The command exited with code {code}"),
            None => "The command was stopped".into(),
        }
    } else {
        message.to_string()
    })
}

impl System for Os {
    fn spawn(&self, job: &Job) -> AppResult<()> {
        let mut child =
            start(login_shell(job), Stdio::null()).map_err(|e| AppError::Launch(e.to_string()))?;
        let stderr = Arc::new(Mutex::new(String::new()));
        let reader = child
            .stderr
            .take()
            .map(|err| collect(err, None, stderr.clone()));
        match exit_by(&mut child, Instant::now() + SPAWN_GRACE) {
            Ok(Some(status)) if status.success() => Ok(()),
            Ok(Some(status)) => {
                if let Some(reader) = reader {
                    let _ = reader.join();
                }
                Err(failure(&stderr.lock_unpoisoned(), status.code()))
            }
            Ok(None) => {
                // Still running: launched. A thread reaps it when it ends.
                thread::spawn(move || {
                    let _ = child.wait();
                });
                Ok(())
            }
            Err(error) => Err(AppError::Launch(error.to_string())),
        }
    }

    fn run(&self, job: &Job, timeout: Duration) -> Output {
        capture(login_shell(job), timeout)
    }

    fn open(&self, args: &[&str]) -> AppResult<()> {
        let mut command = Command::new("/usr/bin/open");
        command.args(args);
        let output = capture(command, TOOL_TIMEOUT);
        if output.ok() {
            Ok(())
        } else {
            Err(failure(&output.output, output.code))
        }
    }

    fn tool(&self, name: &str, args: &[&str]) -> Option<String> {
        let output = capture(program(name, args), TOOL_TIMEOUT);
        output.ok().then_some(output.stdout)
    }

    fn inspect(&self, cwd: Option<&Path>, name: &str, args: &[&str]) -> Output {
        read(cwd, name, args)
    }

    fn copy(&self, text: &str) -> AppResult<()> {
        let mut child = Command::new("/usr/bin/pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Launch(e.to_string()))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| AppError::Launch(e.to_string()))?;
        }
        child.wait().map_err(|e| AppError::Launch(e.to_string()))?;
        Ok(())
    }
}

#[cfg(any(debug_assertions, test, feature = "test-util"))]
pub use recorder::Recorder;

/// The stand-in for the Mac in debug builds and tests.
#[cfg(any(debug_assertions, test, feature = "test-util"))]
mod recorder {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::{Job, Output, System};
    use crate::error::AppResult;
    use crate::lock::MutexExt;

    /// Records what would run instead of running it: the debug build's stand-in and the tests' fake. `run` and
    /// `tool` answer with what `reply` set for the command, else an empty success (`run`'s output then says what
    /// wasn't run).
    #[derive(Default)]
    pub struct Recorder {
        calls: Mutex<Vec<String>>,
        replies: Mutex<Vec<(String, String)>>,
        clipboard: Mutex<Option<String>>,
    }

    impl Recorder {
        /// `run` or `tool` of `command` (a tool's program and arguments joined by spaces) prints `stdout`.
        pub fn reply(&self, command: &str, stdout: &str) {
            self.replies
                .lock_unpoisoned()
                .push((command.into(), stdout.into()));
        }

        /// Every call so far, as `spawn: <command>`, `run: <command>`, `open: <args>`, `tool: <program args>`.
        pub fn calls(&self) -> Vec<String> {
            self.calls.lock_unpoisoned().clone()
        }

        pub fn clipboard(&self) -> Option<String> {
            self.clipboard.lock_unpoisoned().clone()
        }

        fn record(&self, call: String) {
            log::info!("[launch] {call}");
            self.calls.lock_unpoisoned().push(call);
        }

        fn answer(&self, command: &str) -> Option<String> {
            self.replies
                .lock_unpoisoned()
                .iter()
                .find(|(c, _)| c == command)
                .map(|(_, stdout)| stdout.clone())
        }
    }

    impl System for Recorder {
        fn spawn(&self, job: &Job) -> AppResult<()> {
            self.record(format!("spawn: {}", job.command));
            Ok(())
        }

        fn run(&self, job: &Job, _timeout: Duration) -> Output {
            self.record(format!("run: {}", job.command));
            let (stdout, output) = match self.answer(&job.command) {
                Some(reply) => (reply.clone(), reply),
                None => (
                    String::new(),
                    format!(
                        "Not run: `{}`. Set LAUNCHER_REAL_LAUNCH=1 to run it.",
                        job.command
                    ),
                ),
            };
            Output {
                code: Some(0),
                timed_out: false,
                stdout,
                output,
            }
        }

        fn open(&self, args: &[&str]) -> AppResult<()> {
            self.record(format!("open: {}", args.join(" ")));
            Ok(())
        }

        fn tool(&self, program: &str, args: &[&str]) -> Option<String> {
            let call = std::iter::once(program)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            self.record(format!("tool: {call}"));
            self.answer(&call)
        }

        fn inspect(&self, _cwd: Option<&std::path::Path>, program: &str, args: &[&str]) -> Output {
            let call = std::iter::once(program)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            self.record(format!("inspect: {call}"));
            let stdout = self.answer(&call).unwrap_or_default();
            Output {
                code: Some(0),
                timed_out: false,
                output: stdout.clone(),
                stdout,
            }
        }

        fn copy(&self, text: &str) -> AppResult<()> {
            self.record(format!("copy: {text}"));
            *self.clipboard.lock_unpoisoned() = Some(text.into());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recorder_records_an_inspect_and_prints_what_it_was_told_to() {
        let recorder = Recorder::default();
        recorder.reply("ddev describe --json-output", "{\"raw\":{}}");

        let output = recorder.inspect(None, "ddev", &["describe", "--json-output"]);

        assert_eq!(
            output,
            Output {
                code: Some(0),
                timed_out: false,
                stdout: "{\"raw\":{}}".into(),
                output: "{\"raw\":{}}".into(),
            }
        );
        assert_eq!(recorder.calls(), ["inspect: ddev describe --json-output"]);
    }

    #[test]
    fn the_recorder_answers_an_unknown_inspect_with_an_empty_success() {
        let recorder = Recorder::default();

        let output = recorder.inspect(Some(Path::new("/tmp/demo")), "ddev", &["list"]);

        assert_eq!(
            output,
            Output {
                code: Some(0),
                ..Output::default()
            }
        );
    }
}
