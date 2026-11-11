use serde_json::json;

use super::*;
use crate::projects::view::ProjectMeta;

const NOW: i64 = 1_000_000_000_000;

fn project(name: &str) -> Project {
    Project {
        id: name.to_lowercase(),
        path: format!("/work/{}", name.to_lowercase()),
        name: name.into(),
        meta: ProjectMeta {
            icon: "Folder".into(),
            color: "Blue".into(),
            ..ProjectMeta::default()
        },
        added_at: 1000,
        last_opened_at: None,
        available: true,
        locked: false,
        file_error: None,
    }
}

fn tagged(name: &str, tag: &str) -> Project {
    let mut p = project(name);
    p.meta.tag = Some(tag.into());
    p
}

fn starred(name: &str) -> Project {
    let mut p = project(name);
    p.meta.starred = true;
    p
}

fn archived(name: &str) -> Project {
    let mut p = project(name);
    p.meta.archived = true;
    p
}

fn opened(name: &str, at: i64) -> Project {
    let mut p = project(name);
    p.last_opened_at = Some(at);
    p
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn names(library: &Library) -> Vec<(Option<&str>, Vec<&str>)> {
    library
        .sections
        .iter()
        .map(|s| {
            (
                s.title.as_deref(),
                s.rows.iter().map(|r| r.project.name.as_str()).collect(),
            )
        })
        .collect()
}

fn built(projects: &[Project], filter: &Filter, query: &str) -> Library {
    build(projects, &[], filter, query, NOW)
}

#[test]
fn each_filter_shows_the_projects_it_stands_for() {
    let plain = project("Plain");
    let star = starred("Star");
    let tag = tagged("Tag", "Client");
    let other_tag = tagged("Other", "Internal");
    let mut starred_and_tagged = tagged("Both", "Client");
    starred_and_tagged.meta.starred = true;
    let old = archived("Old");
    let mut old_tagged = archived("OldTagged");
    old_tagged.meta.tag = Some("Client".into());
    old_tagged.meta.starred = true;
    let all = [
        plain,
        star,
        tag,
        other_tag,
        starred_and_tagged,
        old,
        old_tagged,
    ];

    let shown = |filter: Filter| -> Vec<&str> {
        all.iter()
            .filter(|p| filter.shows(p))
            .map(|p| p.name.as_str())
            .collect()
    };

    assert_eq!(
        shown(Filter::All),
        ["Plain", "Star", "Tag", "Other", "Both"]
    );
    assert_eq!(shown(Filter::Starred), ["Star", "Both"]);
    assert_eq!(shown(Filter::Untagged), ["Plain", "Star"]);
    assert_eq!(shown(Filter::Archived), ["Old", "OldTagged"]);
    assert_eq!(shown(Filter::Tag("Client".into())), ["Tag", "Both"]);
    assert_eq!(shown(Filter::Tag("Nothing".into())), Vec::<&str>::new());
}

#[test]
fn filter_titles_name_the_entry() {
    for (filter, expected) in [
        (Filter::All, "All Projects"),
        (Filter::Starred, "Starred"),
        (Filter::Untagged, "Untagged"),
        (Filter::Archived, "Archived"),
        (Filter::Tag("Client".into()), "Client"),
    ] {
        assert_eq!(filter.title(), expected, "{filter:?}");
    }
}

#[test]
fn a_filter_serializes_as_a_kind_with_the_tag_only_for_tags() {
    for (filter, expected) in [
        (Filter::All, json!({"kind": "all"})),
        (Filter::Starred, json!({"kind": "starred"})),
        (Filter::Untagged, json!({"kind": "untagged"})),
        (Filter::Archived, json!({"kind": "archived"})),
        (
            Filter::Tag("Client".into()),
            json!({"kind": "tag", "tag": "Client"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&filter).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<Filter>(expected).unwrap(),
            filter,
            "{filter:?}"
        );
    }
}

#[test]
fn tags_are_the_listed_ones_in_order_then_the_extra_ones_alphabetically() {
    let projects = [
        tagged("A", "zeta"),
        tagged("B", "Beta"),
        tagged("C", "alpha"),
        tagged("D", "Listed Two"),
        project("E"),
    ];

    assert_eq!(
        tags(&strings(&["Listed Two", "Listed One"]), &projects),
        ["Listed Two", "Listed One", "alpha", "Beta", "zeta"]
    );
}

#[test]
fn a_tag_several_projects_carry_appears_once() {
    let projects = [
        tagged("A", "Client"),
        tagged("B", "Client"),
        tagged("C", "Other"),
    ];

    assert_eq!(tags(&strings(&["Client"]), &projects), ["Client", "Other"]);
    assert_eq!(tags(&[], &projects), ["Client", "Other"]);
}

#[test]
fn tags_differing_only_in_case_stay_distinct_and_each_appears_once() {
    let projects = [tagged("A", "Zed"), tagged("B", "zed"), tagged("C", "Zed")];

    assert_eq!(tags(&[], &projects), ["Zed", "zed"]);
}

#[test]
fn without_projects_or_listed_tags_there_are_no_tags() {
    assert!(tags(&[], &[]).is_empty());
}

#[test]
fn the_sidebar_counts_projects_per_entry_and_leaves_archived_out_of_the_rest() {
    let mut archived_starred_tagged = tagged("Old", "Client");
    archived_starred_tagged.meta.archived = true;
    archived_starred_tagged.meta.starred = true;
    let mut starred_tagged = tagged("Fav", "Client");
    starred_tagged.meta.starred = true;
    let projects = [
        project("Plain"),
        starred("Star"),
        starred_tagged,
        tagged("Shop", "Client"),
        tagged("Site", "Internal"),
        archived("Gone"),
        archived_starred_tagged,
    ];

    let library = build(
        &projects,
        &strings(&["Client", "Internal", "Empty"]),
        &Filter::All,
        "",
        NOW,
    );

    assert_eq!(
        library.sidebar,
        Sidebar {
            all: 5,
            starred: 2,
            untagged: Some(2),
            archived: 2,
            tags: vec![
                TagCount {
                    name: "Client".into(),
                    count: 2
                },
                TagCount {
                    name: "Internal".into(),
                    count: 1
                },
                TagCount {
                    name: "Empty".into(),
                    count: 0
                },
            ],
        }
    );
}

#[test]
fn untagged_is_hidden_without_tags_and_when_every_project_has_one() {
    let untagged_only = [project("A"), project("B")];
    assert_eq!(
        built(&untagged_only, &Filter::All, "").sidebar.untagged,
        None
    );

    let all_tagged = [tagged("A", "Client")];
    assert_eq!(built(&all_tagged, &Filter::All, "").sidebar.untagged, None);

    let mixed = [tagged("A", "Client"), project("B")];
    assert_eq!(built(&mixed, &Filter::All, "").sidebar.untagged, Some(1));
}

#[test]
fn untagged_is_shown_when_the_tag_list_has_tags_even_if_no_project_carries_them() {
    let library = build(
        &[project("A")],
        &strings(&["Client"]),
        &Filter::All,
        "",
        NOW,
    );
    assert_eq!(library.sidebar.untagged, Some(1));
}

#[test]
fn the_count_label_counts_the_filter_before_searching() {
    let projects = [project("Alpha"), project("Beta"), starred("Gamma")];

    for (filter, query, expected) in [
        (Filter::All, "", "3 projects"),
        (Filter::All, "alpha", "3 projects"),
        (Filter::All, "nothing", "3 projects"),
        (Filter::Starred, "", "1 project"),
        (Filter::Archived, "", "0 projects"),
    ] {
        assert_eq!(
            built(&projects, &filter, query).count_label,
            expected,
            "{filter:?} {query}"
        );
    }
}

#[test]
fn the_library_carries_the_title_of_its_filter() {
    assert_eq!(built(&[], &Filter::Starred, "").title, "Starred");
    assert_eq!(
        built(&[], &Filter::Tag("Client".into()), "").title,
        "Client"
    );
}

#[test]
fn recent_projects_come_first_and_the_rest_follow_each_sorted_by_name_ignoring_case() {
    let projects = [
        opened("zulu", NOW - 1000),
        project("Yankee"),
        opened("Alpha", NOW - 5000),
        project("bravo"),
        opened("Old", NOW - RECENT_MS - 1),
        project("Charlie"),
    ];

    let library = built(&projects, &Filter::All, "");

    assert_eq!(
        names(&library),
        [
            (Some("Recent"), vec!["Alpha", "zulu"]),
            (Some("All"), vec!["bravo", "Charlie", "Old", "Yankee"]),
        ]
    );
}

#[test]
fn a_project_opened_exactly_the_recent_window_ago_is_still_recent() {
    let projects = [
        opened("Edge", NOW - RECENT_MS),
        opened("Past", NOW - RECENT_MS - 1),
    ];

    assert_eq!(
        names(&built(&projects, &Filter::All, "")),
        [(Some("Recent"), vec!["Edge"]), (Some("All"), vec!["Past"]),]
    );
}

#[test]
fn only_non_recent_projects_make_one_untitled_section() {
    let projects = [project("Beta"), project("alpha")];

    assert_eq!(
        names(&built(&projects, &Filter::All, "")),
        [(None, vec!["alpha", "Beta"])]
    );
}

#[test]
fn only_recent_projects_make_just_a_recent_section() {
    let projects = [opened("Beta", NOW), opened("Alpha", NOW - 1)];

    assert_eq!(
        names(&built(&projects, &Filter::All, "")),
        [(Some("Recent"), vec!["Alpha", "Beta"])]
    );
}

#[test]
fn an_empty_filter_has_no_sections_and_says_so() {
    let library = built(&[archived("Old")], &Filter::All, "");

    assert!(library.sections.is_empty());
    assert_eq!(library.empty_text.as_deref(), Some("No projects here"));
    assert_eq!(built(&[project("A")], &Filter::All, "").empty_text, None);
}

#[test]
fn an_empty_search_names_the_trimmed_query_in_its_own_case() {
    let library = built(&[project("Alpha")], &Filter::All, "  NoMatch ");

    assert!(library.sections.is_empty());
    assert_eq!(
        library.empty_text.as_deref(),
        Some("No projects match “NoMatch”")
    );
}

#[test]
fn a_blank_query_matches_everything() {
    let projects = [project("Alpha"), project("Beta")];

    assert_eq!(
        names(&built(&projects, &Filter::All, "   ")),
        [(None, vec!["Alpha", "Beta"])]
    );
}

#[test]
fn search_matches_name_path_tag_and_note_lines_ignoring_case() {
    let mut noted = project("Noted");
    noted.meta.notes = vec!["First line".into(), "Call the CLIENT back".into()];
    let mut pathed = project("Pathed");
    pathed.path = "/work/Special-Folder".into();
    let projects = [
        project("Northwind Shop"),
        pathed,
        tagged("Tagged", "Wholesale"),
        noted,
        project("Unrelated"),
    ];

    for (query, expected) in [
        ("NORTHWIND", vec!["Northwind Shop"]),
        ("special-folder", vec!["Pathed"]),
        ("wholesale", vec!["Tagged"]),
        ("client front", vec![]),
        ("call the client", vec!["Noted"]),
        ("first line", vec!["Noted"]),
    ] {
        let library = built(&projects, &Filter::All, query);
        let found: Vec<&str> = library
            .sections
            .iter()
            .flat_map(|s| s.rows.iter().map(|r| r.project.name.as_str()))
            .collect();
        assert_eq!(found, expected, "{query}");
    }
}

#[test]
fn the_query_is_trimmed_before_matching() {
    let library = built(&[project("Northwind")], &Filter::All, "  north  ");

    assert_eq!(names(&library), [(None, vec!["Northwind"])]);
}

#[test]
fn search_stays_within_the_filter() {
    let projects = [
        starred("Alpha Star"),
        project("Alpha Plain"),
        archived("Alpha Old"),
    ];

    assert_eq!(
        names(&built(&projects, &Filter::Starred, "alpha")),
        [(None, vec!["Alpha Star"])]
    );
    assert_eq!(
        names(&built(&projects, &Filter::Archived, "alpha")),
        [(None, vec!["Alpha Old"])]
    );
    assert_eq!(
        names(&built(&projects, &Filter::Untagged, "old")),
        Vec::<(Option<&str>, Vec<&str>)>::new()
    );
}

#[test]
fn search_does_not_change_the_sidebar_counts() {
    let projects = [project("Alpha"), starred("Beta")];

    let searched = built(&projects, &Filter::All, "alpha");

    assert_eq!(searched.sidebar, built(&projects, &Filter::All, "").sidebar);
    assert_eq!(searched.sidebar.all, 2);
}

#[test]
fn a_row_shows_the_file_problem_first_then_the_first_note_with_text_then_the_folder() {
    let mut broken = project("Broken");
    broken.file_error = Some(".project-launcher.json is missing".into());
    broken.meta.notes = vec!["ignored".into()];
    let mut noted = project("Noted");
    noted.meta.notes = vec![
        "".into(),
        "   ".into(),
        "Ship the beta".into(),
        "later".into(),
    ];
    let plain = project("Plain");

    let library = built(&[broken, noted, plain], &Filter::All, "");

    let subtitles: Vec<(&str, &str)> = library
        .sections
        .iter()
        .flat_map(|s| s.rows.iter())
        .map(|r| (r.project.name.as_str(), r.subtitle.as_str()))
        .collect();
    assert_eq!(
        subtitles,
        [
            ("Broken", ".project-launcher.json is missing"),
            ("Noted", "Ship the beta"),
            ("Plain", "/work/plain"),
        ]
    );
}

#[test]
fn a_library_serializes_with_camel_case_fields() {
    let library = built(&[project("Alpha")], &Filter::All, "");

    let value = serde_json::to_value(&library).unwrap();

    assert_eq!(value["countLabel"], json!("1 project"));
    assert_eq!(value["emptyText"], json!(null));
    assert_eq!(
        value["sidebar"],
        json!({"all": 1, "starred": 0, "untagged": null, "archived": 0, "tags": []})
    );
    assert_eq!(value["sections"][0]["title"], json!(null));
    assert_eq!(
        value["sections"][0]["rows"][0]["subtitle"],
        json!("/work/alpha")
    );
}
