//! The main window's library: which projects a sidebar filter shows, the sidebar's counts and tags, and the
//! list's sections ("Recent", "All"), searched. Pure: the projects, the tag list and the time come in.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::projects::Project;

/// Opened within this long counts as recent.
pub const RECENT_MS: i64 = 72 * 60 * 60 * 1000;

/// A sidebar entry. Archived projects show only under `Archived`.
#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq, Eq, Default)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "kind", content = "tag", rename_all = "camelCase")]
pub enum Filter {
    #[default]
    All,
    Starred,
    Untagged,
    Archived,
    Tag(String),
}

impl Filter {
    pub fn title(&self) -> String {
        match self {
            Filter::All => "All Projects".into(),
            Filter::Starred => "Starred".into(),
            Filter::Untagged => "Untagged".into(),
            Filter::Archived => "Archived".into(),
            Filter::Tag(tag) => tag.clone(),
        }
    }

    pub fn shows(&self, project: &Project) -> bool {
        let meta = &project.meta;
        match self {
            Filter::Archived => meta.archived,
            _ if meta.archived => false,
            Filter::All => true,
            Filter::Starred => meta.starred,
            Filter::Untagged => meta.tag.is_none(),
            Filter::Tag(tag) => meta.tag.as_ref() == Some(tag),
        }
    }
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Library {
    /// The filter's title, for the sidebar's header.
    pub title: String,
    /// "3 projects": how many the filter shows, before searching.
    pub count_label: String,
    pub sidebar: Sidebar,
    /// "Recent" and "All", each alphabetical; one untitled section when only one kind has projects.
    pub sections: Vec<Section>,
    /// Why the list is empty, when it is.
    pub empty_text: Option<String>,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct Sidebar {
    pub all: usize,
    pub starred: usize,
    /// Shown only while there are tags and some project has none.
    pub untagged: Option<usize>,
    pub archived: usize,
    pub tags: Vec<TagCount>,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct TagCount {
    pub name: String,
    pub count: usize,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Section {
    pub title: Option<String>,
    pub rows: Vec<Row>,
}

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
pub struct Row {
    pub project: Project,
    /// The file's problem, else the first note line, else the folder.
    pub subtitle: String,
}

/// The tag list in its order, then tags only projects carry, alphabetically.
fn tags(listed: &[String], projects: &[Project]) -> Vec<String> {
    let mut extra: Vec<String> = projects
        .iter()
        .filter_map(|p| p.meta.tag.clone())
        .filter(|tag| !listed.contains(tag))
        .collect();
    extra.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    extra.dedup();
    listed.iter().cloned().chain(extra).collect()
}

pub fn build(
    projects: &[Project],
    listed_tags: &[String],
    filter: &Filter,
    query: &str,
    now_ms: i64,
) -> Library {
    let shown = projects.iter().filter(|p| filter.shows(p)).count();
    let query = query.trim();
    let needle = query.to_lowercase();
    let mut rows: Vec<&Project> = projects
        .iter()
        .filter(|p| filter.shows(p) && matches(p, &needle))
        .collect();
    rows.sort_by_key(|p| p.name.to_lowercase());
    let (recent, rest): (Vec<&Project>, Vec<&Project>) = rows
        .into_iter()
        .partition(|p| p.last_opened_at.is_some_and(|t| now_ms - t <= RECENT_MS));
    let sections = sections(recent, rest);
    let empty_text = sections.is_empty().then(|| {
        if query.is_empty() {
            "No projects here".to_string()
        } else {
            format!("No projects match “{query}”")
        }
    });
    Library {
        title: filter.title(),
        count_label: match shown {
            1 => "1 project".into(),
            n => format!("{n} projects"),
        },
        sidebar: sidebar(projects, listed_tags),
        sections,
        empty_text,
    }
}

fn sidebar(projects: &[Project], listed_tags: &[String]) -> Sidebar {
    let count = |f: &Filter| projects.iter().filter(|p| f.shows(p)).count();
    let tag_counts: Vec<TagCount> = tags(listed_tags, projects)
        .into_iter()
        .map(|name| TagCount {
            count: count(&Filter::Tag(name.clone())),
            name,
        })
        .collect();
    let untagged = count(&Filter::Untagged);
    Sidebar {
        all: count(&Filter::All),
        starred: count(&Filter::Starred),
        untagged: (!tag_counts.is_empty() && untagged > 0).then_some(untagged),
        archived: count(&Filter::Archived),
        tags: tag_counts,
    }
}

fn sections(recent: Vec<&Project>, rest: Vec<&Project>) -> Vec<Section> {
    let section = |title: Option<&str>, projects: Vec<&Project>| Section {
        title: title.map(str::to_string),
        rows: projects.into_iter().map(row).collect(),
    };
    match (recent.is_empty(), rest.is_empty()) {
        (true, true) => Vec::new(),
        (true, false) => vec![section(None, rest)],
        (false, true) => vec![section(Some("Recent"), recent)],
        (false, false) => vec![section(Some("Recent"), recent), section(Some("All"), rest)],
    }
}

fn row(project: &Project) -> Row {
    let subtitle = match &project.file_error {
        Some(problem) => problem.clone(),
        None => project
            .meta
            .notes
            .iter()
            .find(|line| !line.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| project.path.clone()),
    };
    Row {
        project: project.clone(),
        subtitle,
    }
}

/// A lowercase `query` found in the name, folder, tag or a note line; an empty query matches all.
fn matches(project: &Project, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let found = |text: &str| text.to_lowercase().contains(query);
    found(&project.name)
        || found(&project.path)
        || project.meta.tag.as_deref().is_some_and(found)
        || project.meta.notes.iter().any(|line| found(line))
}

#[cfg(test)]
mod tests;
