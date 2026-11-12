//! The stand-in for the Mac in debug builds and tests.

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
