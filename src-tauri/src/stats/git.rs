//! A repository's branch, changes, last commit, age, latest tag and twelve weeks of commit activity, from
//! read-only `git` commands. A command that fails (no commits, no tags, no upstream) counts as empty.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use crate::run::system::read;

pub const ACTIVITY_WEEKS: usize = 12;
const WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;
const SEPARATOR: char = '\u{1f}';

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub subject: String,
    pub author: String,
    #[ts(type = "number")]
    pub at_ms: i64,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct GitStats {
    /// `None` on a detached HEAD.
    pub branch: Option<String>,
    pub has_upstream: bool,
    pub ahead: u32,
    pub behind: u32,
    pub modified: u32,
    pub untracked: u32,
    pub last_commit: Option<Commit>,
    pub commits: u32,
    /// The first commit's month, e.g. "Mar 2024".
    pub started: Option<String>,
    pub latest_tag: Option<String>,
    /// Commits per week, oldest week first.
    pub weekly: Vec<u32>,
}

/// "Mar 2024" in the local time zone; month names stay English like the rest of the app (ADR 0010).
pub fn month_year(ms: i64) -> Option<String> {
    let time = chrono::DateTime::from_timestamp_millis(ms)?;
    Some(
        time.with_timezone(&chrono::Local)
            .format("%b %Y")
            .to_string(),
    )
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = read(Some(dir), "git", args);
    if output.ok() {
        output.stdout.trim().to_string()
    } else {
        String::new()
    }
}

/// `git status --porcelain=v2 --branch` into the branch, upstream and change counts.
pub fn status(output: &str, stats: &mut GitStats) {
    for line in output.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            stats.branch = Some(head.to_string()).filter(|h| h != "(detached)");
        } else if line.starts_with("# branch.upstream ") {
            stats.has_upstream = true;
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut counts = ab
                .split_whitespace()
                .map(|n| n.trim_start_matches(['+', '-']).parse().unwrap_or(0));
            stats.ahead = counts.next().unwrap_or(0);
            stats.behind = counts.next().unwrap_or(0);
        } else if line.starts_with("? ") {
            stats.untracked += 1;
        } else if line.starts_with("1 ") || line.starts_with("2 ") || line.starts_with("u ") {
            stats.modified += 1;
        }
    }
}

/// Commit times (seconds) into commits per week, oldest first.
pub fn weekly(timestamps: &str, now_seconds: i64) -> Vec<u32> {
    let mut weeks = vec![0; ACTIVITY_WEEKS];
    for at in timestamps
        .lines()
        .filter_map(|t| t.trim().parse::<i64>().ok())
    {
        let ago = (now_seconds - at).div_euclid(WEEK_SECONDS);
        if (0..ACTIVITY_WEEKS as i64).contains(&ago) {
            weeks[ACTIVITY_WEEKS - 1 - ago as usize] += 1;
        }
    }
    weeks
}

/// `subject␟author␟seconds` of `git log -1`.
pub fn last_commit(output: &str) -> Option<Commit> {
    let mut fields = output.split(SEPARATOR);
    let subject = fields.next()?.to_string();
    let author = fields.next()?.to_string();
    let at: i64 = fields.next()?.trim().parse().ok()?;
    Some(Commit {
        subject,
        author,
        at_ms: at * 1000,
    })
}

/// The repository's facts; `None` for a folder that isn't one.
pub fn stats(dir: &Path, now_ms: i64) -> Option<GitStats> {
    if !dir.join(".git").exists() {
        return None;
    }
    let mut stats = GitStats::default();
    status(
        &git(dir, &["status", "--porcelain=v2", "--branch"]),
        &mut stats,
    );
    stats.last_commit = last_commit(&git(
        dir,
        &[
            "log",
            "-1",
            &format!("--format=%s{SEPARATOR}%an{SEPARATOR}%ct"),
        ],
    ));
    stats.commits = git(dir, &["rev-list", "--count", "HEAD"])
        .parse()
        .unwrap_or(0);
    stats.started = git(dir, &["log", "--max-parents=0", "--format=%ct", "HEAD"])
        .lines()
        .filter_map(|t| t.trim().parse::<i64>().ok())
        .min()
        .and_then(|seconds| month_year(seconds * 1000));
    stats.latest_tag =
        Some(git(dir, &["describe", "--tags", "--abbrev=0"])).filter(|t| !t.is_empty());
    let since = format!(
        "--since=@{}",
        now_ms / 1000 - ACTIVITY_WEEKS as i64 * WEEK_SECONDS
    );
    stats.weekly = weekly(&git(dir, &["log", &since, "--format=%ct"]), now_ms / 1000);
    Some(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const DAY_SECONDS: i64 = 24 * 60 * 60;

    #[test]
    fn status_reads_the_branch_upstream_distance_and_changes() {
        let output = "# branch.oid 1234abcd\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 aaaa bbbb src/a.js\n2 R. N... 100644 100644 100644 aaaa bbbb R100 new.js\told.js\nu UU N... 100644 100644 100644 100644 aaaa bbbb cccc conflict.js\n? notes.txt\n? scratch.txt\n";
        let mut stats = GitStats::default();

        status(output, &mut stats);

        assert_eq!(
            stats,
            GitStats {
                branch: Some("main".into()),
                has_upstream: true,
                ahead: 2,
                behind: 1,
                modified: 3,
                untracked: 2,
                ..GitStats::default()
            }
        );
    }

    #[test]
    fn a_detached_head_has_no_branch_and_no_upstream_means_no_distance() {
        let mut stats = GitStats::default();

        status(
            "# branch.oid 1234abcd\n# branch.head (detached)\n",
            &mut stats,
        );

        assert_eq!(stats, GitStats::default());
    }

    #[test]
    fn a_clean_status_counts_nothing() {
        let mut stats = GitStats::default();

        status("", &mut stats);

        assert_eq!(stats, GitStats::default());
    }

    #[test]
    fn weekly_buckets_commits_into_twelve_weeks_oldest_first() {
        let now = 100 * WEEK_SECONDS;
        let times = [
            now,
            now - 1,
            now - WEEK_SECONDS,
            now - 11 * WEEK_SECONDS - 1,
            now - 11 * WEEK_SECONDS - 2,
        ]
        .map(|t| t.to_string())
        .join("\n");

        let weeks = weekly(&times, now);

        assert_eq!(weeks, [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2]);
    }

    #[test]
    fn weekly_ignores_commits_outside_the_window_and_in_the_future() {
        let now = 100 * WEEK_SECONDS;
        let times = format!(
            "{}\n{}\n{}\nnot a time\n\n",
            now - 12 * WEEK_SECONDS,
            now + 10,
            now - 100 * WEEK_SECONDS
        );

        assert_eq!(weekly(&times, now), [0; ACTIVITY_WEEKS]);
    }

    #[test]
    fn weekly_without_commits_is_twelve_empty_weeks() {
        assert_eq!(weekly("", 1_000_000), vec![0; 12]);
    }

    #[test]
    fn the_last_commit_is_read_from_its_separated_fields() {
        assert_eq!(
            last_commit("Fix the pricing currency\u{1f}Demo Developer\u{1f}1700000000\n"),
            Some(Commit {
                subject: "Fix the pricing currency".into(),
                author: "Demo Developer".into(),
                at_ms: 1_700_000_000_000,
            })
        );
    }

    #[test]
    fn a_malformed_or_empty_last_commit_is_none() {
        for output in [
            "",
            "only a subject",
            "subject\u{1f}author",
            "s\u{1f}a\u{1f}soon",
        ] {
            assert_eq!(last_commit(output), None, "{output:?}");
        }
    }

    #[test]
    fn a_folder_without_a_git_directory_has_no_git_stats() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(stats(dir.path(), 1_000_000), None);
    }

    #[test]
    fn month_year_names_the_month_in_english_with_the_year() {
        assert_eq!(month_year(1_710_504_000_000), Some("Mar 2024".to_string()));
    }

    #[test]
    fn month_year_of_an_out_of_range_timestamp_is_none() {
        assert_eq!(month_year(i64::MAX), None);
    }

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn run_git(dir: &Path, args: &[&str], at_seconds: i64) {
        let at = format!("@{at_seconds} +0000");
        let status = Command::new("git")
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
            .unwrap()
            .status;
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn a_repository_reports_its_branch_commits_tag_and_weekly_activity() {
        if !git_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let now_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        run_git(dir.path(), &["init", "-q", "-b", "main"], now_seconds);
        std::fs::write(dir.path().join("a.txt"), "one\n").unwrap();
        run_git(dir.path(), &["add", "-A"], now_seconds);
        let commits = [
            ("First", 10 * DAY_SECONDS),
            ("Second", 3 * DAY_SECONDS),
            ("Third", DAY_SECONDS),
        ];
        for (n, (subject, ago)) in commits.iter().enumerate() {
            let at = now_seconds - ago;
            let empty = if n == 0 { "" } else { "--allow-empty" };
            let args: Vec<&str> = ["commit", "-q", "-m", *subject]
                .into_iter()
                .chain(Some(empty).filter(|e| !e.is_empty()))
                .collect();
            run_git(dir.path(), &args, at);
        }
        run_git(dir.path(), &["tag", "v1.0.0"], now_seconds);
        std::fs::write(dir.path().join("scratch.txt"), "x").unwrap();

        let stats = stats(dir.path(), now_seconds * 1000).unwrap();

        assert_eq!(
            stats,
            GitStats {
                branch: Some("main".into()),
                has_upstream: false,
                ahead: 0,
                behind: 0,
                modified: 0,
                untracked: 1,
                last_commit: Some(Commit {
                    subject: "Third".into(),
                    author: "Demo Developer".into(),
                    at_ms: (now_seconds - DAY_SECONDS) * 1000,
                }),
                commits: 3,
                started: month_year((now_seconds - 10 * DAY_SECONDS) * 1000),
                latest_tag: Some("v1.0.0".into()),
                weekly: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2],
            }
        );
    }
}
