//! A project's files (what git doesn't ignore, else a walk that skips dependency and build folders), their
//! languages, lines and sizes, and the ones edited last.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use ts_rs::TS;

use crate::projects::FILE_NAME;
use crate::run::system::read;

/// The scan stops here (`truncated`).
pub const MAX_FILES: usize = 50_000;
/// Lines are counted in text files up to this size.
const MAX_COUNTED_BYTES: u64 = 1024 * 1024;
const RECENT: usize = 5;
/// Languages listed by name; the rest is "Other".
const TOP_LANGUAGES: usize = 6;
const SKIPPED_FOLDERS: [&str; 12] = [
    ".git",
    "node_modules",
    "vendor",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    ".cache",
    "coverage",
    ".ddev",
    ".idea",
];

/// A file inside the project, by its path relative to the folder.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub bytes: u64,
    pub modified_ms: i64,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Edited {
    pub path: String,
    #[ts(type = "number")]
    pub at_ms: i64,
}

/// One language's share; `color` is a project colour name ("Blue"), `None` for Other.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct LanguageShare {
    pub name: String,
    pub color: Option<String>,
    pub files: u32,
    #[ts(type = "number")]
    pub lines: u64,
    #[ts(type = "number")]
    pub bytes: u64,
}

struct Language {
    name: &'static str,
    color: &'static str,
    /// Text files get their lines counted.
    text: bool,
}

const fn lang(name: &'static str, color: &'static str) -> Language {
    Language {
        name,
        color,
        text: true,
    }
}

const fn binary(name: &'static str, color: &'static str) -> Language {
    Language {
        name,
        color,
        text: false,
    }
}

fn language(extension: &str) -> Option<Language> {
    Some(match extension {
        "php" => lang("PHP", "Purple"),
        "js" | "mjs" | "cjs" | "jsx" => lang("JavaScript", "Yellow"),
        "ts" | "mts" | "tsx" => lang("TypeScript", "Blue"),
        "vue" => lang("Vue", "Green"),
        "svelte" => lang("Svelte", "Orange"),
        "css" => lang("CSS", "Magenta"),
        "scss" | "sass" => lang("Sass", "Magenta"),
        "less" => lang("Less", "Magenta"),
        "html" | "htm" => lang("HTML", "Orange"),
        "twig" => lang("Twig", "Green"),
        "json" => lang("JSON", "Yellow"),
        "yml" | "yaml" => lang("YAML", "Red"),
        "xml" => lang("XML", "Orange"),
        "md" | "mdx" => lang("Markdown", "Blue"),
        "txt" => lang("Text", "Blue"),
        "sh" | "bash" | "zsh" => lang("Shell", "Green"),
        "py" => lang("Python", "Blue"),
        "rb" => lang("Ruby", "Red"),
        "go" => lang("Go", "Blue"),
        "rs" => lang("Rust", "Orange"),
        "java" => lang("Java", "Orange"),
        "kt" => lang("Kotlin", "Purple"),
        "swift" => lang("Swift", "Orange"),
        "c" | "h" => lang("C", "Blue"),
        "cpp" | "hpp" | "cc" => lang("C++", "Magenta"),
        "sql" => lang("SQL", "Orange"),
        "po" | "pot" => lang("Translations", "Green"),
        "svg" => lang("SVG", "Yellow"),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => binary("Images", "Purple"),
        "woff" | "woff2" | "ttf" | "otf" | "eot" => binary("Fonts", "Red"),
        _ => return None,
    })
}

/// Lock files, minified files, source maps and the project file say nothing about the code.
fn countable(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let lock = name.ends_with(".lock")
        || name.ends_with("-lock.json")
        || name.ends_with("-lock.yaml")
        || name == "bun.lockb";
    !lock && !name.contains(".min.") && !name.ends_with(".map") && name != FILE_NAME
}

fn source_file(dir: &Path, path: String) -> Option<SourceFile> {
    let meta = fs::metadata(dir.join(&path)).ok().filter(|m| m.is_file())?;
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64);
    Some(SourceFile {
        path,
        bytes: meta.len(),
        modified_ms,
    })
}

/// What `git ls-files` lists (tracked and untracked, not ignored); `None` outside a repository.
fn git_files(dir: &Path) -> Option<Vec<String>> {
    if !dir.join(".git").exists() {
        return None;
    }
    let output = read(
        Some(dir),
        "git",
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    );
    output.ok().then(|| {
        output
            .stdout
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
}

/// Every file under `dir`, skipping dependency and build folders, at most `MAX_FILES + 1`.
fn walk(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders: Vec<PathBuf> = vec![PathBuf::new()];
    while let Some(relative) = folders.pop() {
        let Ok(entries) = fs::read_dir(dir.join(&relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = relative.join(&name);
            match entry.file_type() {
                Ok(t) if t.is_dir() && !SKIPPED_FOLDERS.contains(&name.as_str()) => {
                    folders.push(path)
                }
                Ok(t) if t.is_file() => found.push(path.to_string_lossy().into_owned()),
                _ => {}
            }
            if found.len() > MAX_FILES {
                return found;
            }
        }
    }
    found
}

/// The project's files and whether there were more than `MAX_FILES`.
pub fn scan(dir: &Path) -> (Vec<SourceFile>, bool) {
    let mut paths = git_files(dir).unwrap_or_else(|| walk(dir));
    let truncated = paths.len() > MAX_FILES;
    paths.truncate(MAX_FILES);
    (
        paths
            .into_iter()
            .filter_map(|p| source_file(dir, p))
            .collect(),
        truncated,
    )
}

fn line_count(file: &Path) -> u64 {
    fs::read(file).map_or(0, |bytes| {
        let newlines = bytes.iter().filter(|&&b| b == b'\n').count() as u64;
        newlines + u64::from(!bytes.is_empty() && bytes.last() != Some(&b'\n'))
    })
}

/// Lines of the counted text files and the languages, the six largest by name and the rest as Other.
pub fn languages(dir: &Path, files: &[SourceFile]) -> (u64, Vec<LanguageShare>) {
    let mut by_name: HashMap<&'static str, LanguageShare> = HashMap::new();
    let mut total_lines = 0;
    for file in files.iter().filter(|f| countable(&f.path)) {
        let extension = file
            .path
            .rsplit_once('.')
            .map(|(_, e)| e.to_lowercase())
            .unwrap_or_default();
        let Some(language) = language(&extension) else {
            continue;
        };
        let lines = if language.text && file.bytes <= MAX_COUNTED_BYTES {
            line_count(&dir.join(&file.path))
        } else {
            0
        };
        total_lines += lines;
        let share = by_name
            .entry(language.name)
            .or_insert_with(|| LanguageShare {
                name: language.name.into(),
                color: Some(language.color.into()),
                files: 0,
                lines: 0,
                bytes: 0,
            });
        share.files += 1;
        share.lines += lines;
        share.bytes += file.bytes;
    }
    let mut shares: Vec<LanguageShare> = by_name.into_values().collect();
    shares.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    // Folding a single language into Other would hide it for nothing.
    if shares.len() > TOP_LANGUAGES + 1 {
        let rest = shares.split_off(TOP_LANGUAGES);
        shares.push(LanguageShare {
            name: "Other".into(),
            color: None,
            files: rest.iter().map(|s| s.files).sum(),
            lines: rest.iter().map(|s| s.lines).sum(),
            bytes: rest.iter().map(|s| s.bytes).sum(),
        });
    }
    (total_lines, shares)
}

/// The files edited last, newest first; the project file doesn't count.
pub fn recent(files: &[SourceFile]) -> Vec<Edited> {
    let mut edited: Vec<&SourceFile> = files.iter().filter(|f| f.path != FILE_NAME).collect();
    edited.sort_by(|a, b| {
        b.modified_ms
            .cmp(&a.modified_ms)
            .then_with(|| a.path.cmp(&b.path))
    });
    edited
        .into_iter()
        .take(RECENT)
        .map(|f| Edited {
            path: f.path.clone(),
            at_ms: f.modified_ms,
        })
        .collect()
}

/// The folder's size on disk, ignored files included (`du`); `None` when it can't tell.
pub fn disk_bytes(dir: &Path) -> Option<u64> {
    let output = read(None, "du", &["-sk", &dir.to_string_lossy()]);
    let kilobytes: u64 = output.stdout.split_whitespace().next()?.parse().ok()?;
    Some(kilobytes * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn folder(files: &[(&str, &[u8])]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let file = dir.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, content).unwrap();
        }
        dir
    }

    fn share(name: &str, color: Option<&str>, files: u32, lines: u64, bytes: u64) -> LanguageShare {
        LanguageShare {
            name: name.into(),
            color: color.map(String::from),
            files,
            lines,
            bytes,
        }
    }

    fn languages_of(files: &[(&str, &[u8])]) -> (u64, Vec<LanguageShare>) {
        let dir = folder(files);
        let (scanned, _) = scan(dir.path());
        languages(dir.path(), &scanned)
    }

    #[test]
    fn scanning_a_folder_without_git_lists_nested_files_and_skips_dependency_and_build_folders() {
        let dir = folder(&[
            ("a.txt", b"hello"),
            ("src/b.js", b"let b;\n"),
            ("src/deep/c.css", b"x"),
            ("node_modules/pkg/index.js", b"x"),
            ("vendor/autoload.php", b"x"),
            ("dist/bundle.js", b"x"),
            (".git/config", b"x"),
        ]);

        let (mut files, truncated) = scan(dir.path());
        files.sort_by(|a, b| a.path.cmp(&b.path));

        assert!(!truncated);
        let listed: Vec<(&str, u64)> = files.iter().map(|f| (f.path.as_str(), f.bytes)).collect();
        assert_eq!(
            listed,
            [("a.txt", 5), ("src/b.js", 7), ("src/deep/c.css", 1)]
        );
    }

    #[test]
    fn a_scan_of_an_empty_folder_finds_nothing() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(scan(dir.path()), (vec![], false));
    }

    #[test]
    fn lines_are_counted_in_text_files_with_or_without_a_final_newline() {
        let (lines, shares) =
            languages_of(&[("a.js", b"a\nb\n"), ("b.php", b"x\ny"), ("empty.css", b"")]);

        assert_eq!(lines, 4);
        assert_eq!(
            shares,
            [
                share("JavaScript", Some("Yellow"), 1, 2, 4),
                share("PHP", Some("Purple"), 1, 2, 3),
                share("CSS", Some("Magenta"), 1, 0, 0),
            ]
        );
    }

    #[test]
    fn lock_files_minified_files_source_maps_and_unknown_types_are_left_out() {
        let (lines, shares) = languages_of(&[
            ("app.js", b"one\n"),
            ("app.min.js", b"minified\n"),
            ("app.js.map", b"{}\n"),
            ("package-lock.json", b"{}\n"),
            ("pnpm-lock.yaml", b"a: 1\n"),
            ("composer.lock", b"{}\n"),
            ("bun.lockb", b"\0\0"),
            ("Makefile", b"all:\n"),
            ("data.bin", b"\0\0\0"),
        ]);

        assert_eq!(lines, 1);
        assert_eq!(shares, [share("JavaScript", Some("Yellow"), 1, 1, 4)]);
    }

    #[test]
    fn binary_images_count_as_files_without_lines() {
        let (lines, shares) = languages_of(&[("logo.png", b"\x89PNG\r\n\x1a\n")]);

        assert_eq!(lines, 0);
        assert_eq!(shares, [share("Images", Some("Purple"), 1, 0, 8)]);
    }

    #[test]
    fn languages_are_sorted_by_bytes_and_the_extension_is_case_insensitive() {
        let (_, shares) = languages_of(&[
            ("small.PHP", b"x"),
            ("big.ts", b"xxxxxxxx"),
            ("middle.css", b"xxxx"),
        ]);

        let names: Vec<&str> = shares.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["TypeScript", "CSS", "PHP"]);
    }

    #[test]
    fn languages_beyond_the_six_largest_fold_into_other_without_a_color() {
        let (lines, shares) = languages_of(&[
            ("a.php", b"xxxxxxxx"),
            ("b.js", b"xxxxxxx"),
            ("c.ts", b"xxxxxx"),
            ("d.css", b"xxxxx"),
            ("e.html", b"xxxx"),
            ("f.json", b"xxx"),
            ("g.md", b"xx"),
            ("h.py", b"x"),
        ]);

        assert_eq!(lines, 8);
        assert_eq!(
            shares,
            [
                share("PHP", Some("Purple"), 1, 1, 8),
                share("JavaScript", Some("Yellow"), 1, 1, 7),
                share("TypeScript", Some("Blue"), 1, 1, 6),
                share("CSS", Some("Magenta"), 1, 1, 5),
                share("HTML", Some("Orange"), 1, 1, 4),
                share("JSON", Some("Yellow"), 1, 1, 3),
                share("Other", None, 2, 2, 3),
            ]
        );
    }

    #[test]
    fn exactly_seven_languages_are_all_listed_by_name() {
        let (_, shares) = languages_of(&[
            ("a.php", b"xxxxxxx"),
            ("b.js", b"xxxxxx"),
            ("c.ts", b"xxxxx"),
            ("d.css", b"xxxx"),
            ("e.html", b"xxx"),
            ("f.json", b"xx"),
            ("g.md", b"x"),
        ]);

        let names: Vec<&str> = shares.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "PHP",
                "JavaScript",
                "TypeScript",
                "CSS",
                "HTML",
                "JSON",
                "Markdown"
            ]
        );
    }

    fn touched(dir: &Path, path: &str, seconds: u64) {
        let file = fs::File::options()
            .write(true)
            .open(dir.join(path))
            .unwrap();
        file.set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
            .unwrap();
    }

    #[test]
    fn recent_lists_the_five_newest_files_first_and_leaves_out_the_project_file() {
        let dir = folder(&[
            ("f1.txt", b"x"),
            ("f2.txt", b"x"),
            ("f3.txt", b"x"),
            ("sub/f4.txt", b"x"),
            ("f5.txt", b"x"),
            ("f6.txt", b"x"),
            ("f7.txt", b"x"),
            (FILE_NAME, b"{}"),
        ]);
        for (n, path) in [
            "f1.txt",
            "f2.txt",
            "f3.txt",
            "sub/f4.txt",
            "f5.txt",
            "f6.txt",
            "f7.txt",
        ]
        .into_iter()
        .enumerate()
        {
            touched(dir.path(), path, 1000 * (n as u64 + 1));
        }
        touched(dir.path(), FILE_NAME, 99_000);
        let (scanned, _) = scan(dir.path());

        let edited = recent(&scanned);

        let listed: Vec<(&str, i64)> = edited.iter().map(|e| (e.path.as_str(), e.at_ms)).collect();
        assert_eq!(
            listed,
            [
                ("f7.txt", 7_000_000),
                ("f6.txt", 6_000_000),
                ("f5.txt", 5_000_000),
                ("sub/f4.txt", 4_000_000),
                ("f3.txt", 3_000_000),
            ]
        );
    }

    #[test]
    fn recent_with_only_the_project_file_is_empty() {
        let dir = folder(&[(FILE_NAME, b"{}")]);
        let (scanned, _) = scan(dir.path());

        assert_eq!(recent(&scanned), vec![]);
    }

    #[test]
    fn language_shares_serialize_with_camel_case_names() {
        assert_eq!(
            serde_json::to_value(share("Other", None, 2, 3, 4)).unwrap(),
            serde_json::json!({
                "name": "Other",
                "color": null,
                "files": 2,
                "lines": 3,
                "bytes": 4,
            })
        );
        assert_eq!(
            serde_json::to_value(Edited {
                path: "a.txt".into(),
                at_ms: 5
            })
            .unwrap(),
            serde_json::json!({ "path": "a.txt", "atMs": 5 })
        );
    }
}
