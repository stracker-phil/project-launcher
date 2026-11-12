//! The Project menu's items as the palette lists them for the project the main window shows: only the ones that
//! apply, labelled by its state (Unstar, Unarchive, Unlock), with "Move to Tag" for every other tag. The menu bar
//! keeps its fixed labels.

use super::palette::MenuCommand;

/// What the Project menu's items depend on.
pub struct ProjectState {
    /// Its file can be read.
    pub available: bool,
    pub locked: bool,
    pub starred: bool,
    pub archived: bool,
    /// A database DataBee can open.
    pub has_database: bool,
    pub tag: Option<String>,
    /// Every tag, in the sidebar's order.
    pub tags: Vec<String>,
}

const PROJECT_MENU: &str = "Project";
const LOCK: &str = "page:project.lock";
/// Puts the project's tag field into edit mode (the right-click's "New Tag…").
const NEW_TAG: &str = "page:project.newTag";
/// `page:tag:<json>` moves the current project to the tag (`null`: none).
const MOVE_TO_TAG: &str = "page:tag:";

/// Its label for the project, or `None` when it doesn't apply; the page refuses the same.
fn label_for(command: &str, label: &str, project: &ProjectState) -> Option<String> {
    let editable = project.available && !project.locked;
    let applies = match command {
        "page:project.rename" | "page:project.icon" | "page:project.notes" => editable,
        "page:project.star"
        | "page:project.archive"
        | LOCK
        | "page:project.terminal"
        | "page:project.editFile" => project.available,
        "page:project.database" => project.available && project.has_database,
        _ => true,
    };
    let label = match command {
        "page:project.star" if project.starred => "Unstar Project",
        "page:project.archive" if project.archived => "Unarchive Project",
        LOCK if project.locked => "Unlock Project Details",
        _ => label,
    };
    applies.then(|| label.to_string())
}

fn project_item(label: String, command: String) -> MenuCommand {
    MenuCommand {
        menu: PROJECT_MENU.into(),
        label,
        command,
        key: None,
    }
}

fn move_to(tag: Option<&str>) -> String {
    let json = serde_json::to_string(&tag).unwrap_or_else(|_| "null".into());
    format!("{MOVE_TO_TAG}{json}")
}

/// "Move to Tag: <tag>" for every other tag, "Move to Tag: No Tag" while it has one, then "New Tag for
/// Project…"; nothing while its details can't be edited.
fn tag_items(project: &ProjectState) -> Vec<MenuCommand> {
    if !project.available || project.locked {
        return Vec::new();
    }
    let current = project.tag.as_deref();
    let others = project
        .tags
        .iter()
        .filter(|tag| Some(tag.as_str()) != current)
        .map(|tag| project_item(format!("Move to Tag: {tag}"), move_to(Some(tag))));
    let untag = current.map(|_| project_item("Move to Tag: No Tag".into(), move_to(None)));
    others
        .chain(untag)
        .chain([project_item("New Tag for Project…".into(), NEW_TAG.into())])
        .collect()
}

/// The menus' items with the Project menu's fitted to the project; the tag items follow Lock.
pub fn for_project(listed: Vec<MenuCommand>, project: &ProjectState) -> Vec<MenuCommand> {
    let mut fitted = Vec::with_capacity(listed.len());
    for item in listed {
        if item.menu != PROJECT_MENU {
            fitted.push(item);
            continue;
        }
        let is_lock = item.command == LOCK;
        if let Some(label) = label_for(&item.command, &item.label, project) {
            fitted.push(MenuCommand { label, ..item });
        }
        if is_lock {
            fitted.extend(tag_items(project));
        }
    }
    fitted
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT_COMMANDS: [(&str, &str); 9] = [
        ("page:project.rename", "Rename Project"),
        ("page:project.icon", "Choose Icon"),
        ("page:project.notes", "Edit Note"),
        ("page:project.star", "Star Project"),
        ("page:project.archive", "Archive Project"),
        ("page:project.database", "Open Database"),
        ("page:project.terminal", "Open Terminal"),
        ("page:project.editFile", "Edit Project File"),
        (LOCK, "Lock Project Details"),
    ];

    fn item(menu: &str, label: &str, command: &str) -> MenuCommand {
        MenuCommand {
            menu: menu.into(),
            label: label.into(),
            command: command.into(),
            key: None,
        }
    }

    fn listed() -> Vec<MenuCommand> {
        PROJECT_COMMANDS
            .iter()
            .map(|(command, label)| item(PROJECT_MENU, label, command))
            .collect()
    }

    fn state() -> ProjectState {
        ProjectState {
            available: true,
            locked: false,
            starred: false,
            archived: false,
            has_database: true,
            tag: None,
            tags: Vec::new(),
        }
    }

    fn labels(project: &ProjectState) -> Vec<String> {
        for_project(listed(), project)
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    fn commands(project: &ProjectState) -> Vec<String> {
        for_project(listed(), project)
            .into_iter()
            .map(|item| item.command)
            .collect()
    }

    fn tags(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn an_editable_project_keeps_every_item_with_its_plain_labels() {
        let project = ProjectState {
            tags: Vec::new(),
            ..state()
        };

        let labels = labels(&project);

        assert_eq!(
            labels,
            [
                "Rename Project",
                "Choose Icon",
                "Edit Note",
                "Star Project",
                "Archive Project",
                "Open Database",
                "Open Terminal",
                "Edit Project File",
                "Lock Project Details",
                "New Tag for Project…",
            ]
        );
    }

    #[test]
    fn rename_icon_and_note_are_hidden_while_locked_or_unreadable() {
        let locked = ProjectState {
            locked: true,
            ..state()
        };
        let unreadable = ProjectState {
            available: false,
            ..state()
        };
        for (name, project) in [("locked", locked), ("unreadable", unreadable)] {
            let commands = commands(&project);

            for hidden in [
                "page:project.rename",
                "page:project.icon",
                "page:project.notes",
            ] {
                assert!(
                    !commands.iter().any(|c| c == hidden),
                    "{hidden} when {name}"
                );
            }
        }
    }

    #[test]
    fn a_locked_project_still_offers_star_archive_and_unlock() {
        let project = ProjectState {
            locked: true,
            ..state()
        };

        assert_eq!(
            labels(&project),
            [
                "Star Project",
                "Archive Project",
                "Open Database",
                "Open Terminal",
                "Edit Project File",
                "Unlock Project Details",
            ]
        );
    }

    #[test]
    fn an_unreadable_project_keeps_only_the_items_that_need_no_file() {
        let project = ProjectState {
            available: false,
            ..state()
        };

        assert_eq!(labels(&project), Vec::<String>::new());
    }

    #[test]
    fn open_database_needs_a_database() {
        for (has_database, shown) in [(true, true), (false, false)] {
            let project = ProjectState {
                has_database,
                ..state()
            };

            assert_eq!(
                commands(&project)
                    .iter()
                    .any(|c| c == "page:project.database"),
                shown,
                "has_database: {has_database}"
            );
        }
    }

    #[test]
    fn labels_follow_the_projects_star_archive_and_lock_state() {
        let project = ProjectState {
            starred: true,
            archived: true,
            locked: true,
            ..state()
        };

        let labels = labels(&project);

        for expected in [
            "Unstar Project",
            "Unarchive Project",
            "Unlock Project Details",
        ] {
            assert!(labels.iter().any(|l| l == expected), "{expected}");
        }
        for plain in ["Star Project", "Archive Project", "Lock Project Details"] {
            assert!(!labels.iter().any(|l| l == plain), "{plain}");
        }
    }

    #[test]
    fn tag_rows_follow_lock_for_every_tag_except_the_current_one() {
        let project = ProjectState {
            tag: Some("Work".into()),
            tags: tags(&["Work", "Personal", "Client"]),
            ..state()
        };

        let items = for_project(listed(), &project);
        let after_lock: Vec<(&str, &str)> = items
            .iter()
            .skip_while(|i| i.command != LOCK)
            .skip(1)
            .map(|i| (i.label.as_str(), i.command.as_str()))
            .collect();

        assert_eq!(
            after_lock,
            [
                ("Move to Tag: Personal", "page:tag:\"Personal\""),
                ("Move to Tag: Client", "page:tag:\"Client\""),
                ("Move to Tag: No Tag", "page:tag:null"),
                ("New Tag for Project…", "page:project.newTag"),
            ]
        );
    }

    #[test]
    fn move_to_no_tag_shows_only_for_a_tagged_project() {
        let untagged = ProjectState {
            tags: tags(&["Work"]),
            ..state()
        };
        let tagged = ProjectState {
            tag: Some("Work".into()),
            tags: tags(&["Work"]),
            ..state()
        };

        assert!(!labels(&untagged).iter().any(|l| l == "Move to Tag: No Tag"));
        assert!(labels(&tagged).iter().any(|l| l == "Move to Tag: No Tag"));
    }

    #[test]
    fn tag_rows_are_menu_items_of_the_project_section_without_keys() {
        let project = ProjectState {
            tags: tags(&["Work"]),
            ..state()
        };

        let items = for_project(listed(), &project);

        assert!(items.contains(&item(
            PROJECT_MENU,
            "Move to Tag: Work",
            "page:tag:\"Work\""
        )));
    }

    #[test]
    fn tag_names_with_quotes_are_escaped_in_the_command_id() {
        let project = ProjectState {
            tags: tags(&["Side \"Gigs\""]),
            ..state()
        };

        assert!(commands(&project).contains(&"page:tag:\"Side \\\"Gigs\\\"\"".to_string()));
    }

    #[test]
    fn no_tag_rows_while_locked_or_unavailable() {
        let locked = ProjectState {
            locked: true,
            tag: Some("Work".into()),
            tags: tags(&["Work", "Personal"]),
            ..state()
        };
        let unavailable = ProjectState {
            available: false,
            ..locked_copy()
        };
        for (name, project) in [("locked", locked), ("unavailable", unavailable)] {
            let labels = labels(&project);

            assert!(
                !labels
                    .iter()
                    .any(|l| l.starts_with("Move to Tag") || l == "New Tag for Project…"),
                "{name}"
            );
        }
    }

    fn locked_copy() -> ProjectState {
        ProjectState {
            tag: Some("Work".into()),
            tags: tags(&["Work", "Personal"]),
            ..state()
        }
    }

    #[test]
    fn items_of_other_menus_pass_through_untouched() {
        let other = MenuCommand {
            key: Some("⌘N".into()),
            ..item("File", "New Project", "page:project.new")
        };
        let rename_elsewhere = item("Edit", "Rename Project", "page:project.rename");
        let mut input = listed();
        input.insert(0, other.clone());
        input.push(rename_elsewhere.clone());
        let project = ProjectState {
            locked: true,
            ..state()
        };

        let items = for_project(input, &project);

        assert_eq!(items.first(), Some(&other));
        assert_eq!(items.last(), Some(&rename_elsewhere));
    }
}
