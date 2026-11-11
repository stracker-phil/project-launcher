//! Project stats for the inspector, in two speeds: quick (manifests and git) and files (a scan with languages,
//! lines and disk usage). Each is cached per project for a while; `refresh` reads again.

pub mod commands;
pub mod files;
pub mod git;
pub mod manifests;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use ts_rs::TS;

use crate::lock::MutexExt;
use files::{Edited, LanguageShare};
use git::GitStats;
use manifests::PackageScript;

/// How long stats count as fresh.
const FRESH_MS: i64 = 5 * 60 * 1000;

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct QuickStats {
    /// "WordPress plugin · v3.1.2".
    pub kind: Option<String>,
    pub requirements: Vec<String>,
    /// "43 npm · 20 composer".
    pub packages: Option<String>,
    pub scripts: Vec<PackageScript>,
    pub git: Option<GitStats>,
    #[ts(type = "number")]
    pub computed_at_ms: i64,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct FileStats {
    pub files: u32,
    /// More files than the scan reads.
    pub truncated: bool,
    #[ts(type = "number")]
    pub source_bytes: u64,
    /// Ignored files included.
    #[ts(type = "number | null")]
    pub disk_bytes: Option<u64>,
    #[ts(type = "number")]
    pub lines: u64,
    pub languages: Vec<LanguageShare>,
    /// Newest first.
    pub recent: Vec<Edited>,
    #[ts(type = "number")]
    pub computed_at_ms: i64,
}

pub fn quick(dir: &Path, now_ms: i64) -> QuickStats {
    let facts = manifests::facts(dir);
    let kind = match (facts.kind, facts.version) {
        (Some(kind), Some(version)) => {
            Some(format!("{kind} · v{}", version.trim_start_matches('v')))
        }
        (kind, _) => kind,
    };
    QuickStats {
        kind,
        requirements: facts.requirements,
        packages: facts.packages,
        scripts: facts.scripts,
        git: git::stats(dir, now_ms),
        computed_at_ms: now_ms,
    }
}

pub fn file_stats(dir: &Path, now_ms: i64) -> FileStats {
    let (scanned, truncated) = files::scan(dir);
    let (lines, languages) = files::languages(dir, &scanned);
    FileStats {
        files: scanned.len() as u32,
        truncated,
        source_bytes: scanned.iter().map(|f| f.bytes).sum(),
        disk_bytes: files::disk_bytes(dir),
        lines,
        languages,
        recent: files::recent(&scanned),
        computed_at_ms: now_ms,
    }
}

/// The last stats per project id, kept while fresh.
#[derive(Default)]
pub struct Cache {
    quick: Mutex<HashMap<String, QuickStats>>,
    files: Mutex<HashMap<String, FileStats>>,
}

fn cached<T: Clone>(
    map: &Mutex<HashMap<String, T>>,
    id: &str,
    at: impl Fn(&T) -> i64,
    now_ms: i64,
    refresh: bool,
    read: impl FnOnce() -> T,
) -> T {
    if !refresh
        && let Some(stats) = map
            .lock_unpoisoned()
            .get(id)
            .filter(|s| now_ms - at(s) < FRESH_MS)
    {
        return stats.clone();
    }
    let stats = read();
    map.lock_unpoisoned().insert(id.to_string(), stats.clone());
    stats
}

impl Cache {
    pub fn quick(&self, id: &str, dir: &Path, now_ms: i64, refresh: bool) -> QuickStats {
        cached(
            &self.quick,
            id,
            |s| s.computed_at_ms,
            now_ms,
            refresh,
            || quick(dir, now_ms),
        )
    }

    pub fn files(&self, id: &str, dir: &Path, now_ms: i64, refresh: bool) -> FileStats {
        cached(
            &self.files,
            id,
            |s| s.computed_at_ms,
            now_ms,
            refresh,
            || file_stats(dir, now_ms),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const NOW: i64 = 1_800_000_000_000;

    fn package(version: &str) -> String {
        format!(r#"{{"name":"demo-app","version":"{version}"}}"#)
    }

    #[test]
    fn the_kind_line_joins_kind_and_version_without_doubling_a_leading_v() {
        for (version, expected) in [
            ("2.4.0", "Node package · v2.4.0"),
            ("v2.4.0", "Node package · v2.4.0"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("package.json"), package(version)).unwrap();

            assert_eq!(
                quick(dir.path(), NOW).kind.as_deref(),
                Some(expected),
                "{version}"
            );
        }
    }

    #[test]
    fn the_kind_line_is_just_the_kind_without_a_version() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        assert_eq!(quick(dir.path(), NOW).kind.as_deref(), Some("Node package"));
    }

    #[test]
    fn quick_stats_carry_the_package_counts_of_the_manifests() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"react":"^19"}}"#,
        )
        .unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        fs::write(
            dir.path().join("composer.json"),
            r#"{"require":{"monolog/monolog":"^3","php":">=8.2"}}"#,
        )
        .unwrap();

        assert_eq!(
            quick(dir.path(), NOW).packages.as_deref(),
            Some("1 yarn · 1 composer")
        );
    }

    #[test]
    fn a_plain_folder_has_quick_stats_without_kind_scripts_or_git() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            quick(dir.path(), NOW),
            QuickStats {
                kind: None,
                requirements: vec![],
                packages: None,
                scripts: vec![],
                git: None,
                computed_at_ms: NOW,
            }
        );
    }

    #[test]
    fn file_stats_total_the_files_bytes_and_lines() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.js"), "a\nb\n").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/b.css"), "x\n").unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/skipped.js"), "x\n").unwrap();

        let stats = file_stats(dir.path(), NOW);

        assert_eq!(stats.files, 2);
        assert!(!stats.truncated);
        assert_eq!(stats.source_bytes, 6);
        assert_eq!(stats.lines, 3);
        assert_eq!(stats.computed_at_ms, NOW);
        let languages: Vec<(&str, u64)> = stats
            .languages
            .iter()
            .map(|l| (l.name.as_str(), l.lines))
            .collect();
        assert_eq!(languages, [("JavaScript", 2), ("CSS", 1)]);
        assert_eq!(stats.recent.len(), 2);
    }

    #[test]
    fn cached_file_stats_stay_until_refreshed_or_stale() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        let cache = Cache::default();
        assert_eq!(cache.files("p", dir.path(), NOW, false).files, 1);
        fs::write(dir.path().join("b.txt"), "b").unwrap();

        let within_fresh = cache.files("p", dir.path(), NOW + FRESH_MS - 1, false);
        let refreshed = cache.files("p", dir.path(), NOW + 10, true);
        let after_refresh = cache.files("p", dir.path(), NOW + 20, false);

        assert_eq!(within_fresh.files, 1);
        assert_eq!(refreshed.files, 2);
        assert_eq!(refreshed.computed_at_ms, NOW + 10);
        assert_eq!(after_refresh, refreshed);
    }

    #[test]
    fn stale_file_stats_are_read_again() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        let cache = Cache::default();
        cache.files("p", dir.path(), NOW, false);
        fs::write(dir.path().join("b.txt"), "b").unwrap();

        let stale = cache.files("p", dir.path(), NOW + FRESH_MS, false);

        assert_eq!(stale.files, 2);
    }

    #[test]
    fn cached_quick_stats_stay_until_refreshed_and_are_kept_per_project() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("package.json");
        fs::write(&file, package("1.0.0")).unwrap();
        let cache = Cache::default();
        cache.quick("p", dir.path(), NOW, false);
        fs::write(&file, package("2.0.0")).unwrap();
        let kind = |stats: QuickStats| stats.kind.unwrap();

        let cached = cache.quick("p", dir.path(), NOW + 1, false);
        let other_project = cache.quick("q", dir.path(), NOW + 1, false);
        let refreshed = cache.quick("p", dir.path(), NOW + 2, true);

        assert_eq!(kind(cached), "Node package · v1.0.0");
        assert_eq!(kind(other_project), "Node package · v2.0.0");
        assert_eq!(kind(refreshed), "Node package · v2.0.0");
    }

    #[test]
    fn stats_serialize_with_camel_case_names() {
        let value = serde_json::to_value(quick(tempfile::tempdir().unwrap().path(), NOW)).unwrap();

        assert_eq!(
            value,
            serde_json::json!({
                "kind": null,
                "requirements": [],
                "packages": null,
                "scripts": [],
                "git": null,
                "computedAtMs": NOW,
            })
        );
    }
}
