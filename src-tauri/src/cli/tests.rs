//! The command line's parsing: which words become which request, and how replies print.
use super::*;
use crate::automation::import::{Plan, Skipped};
use crate::automation::{Description, ListedProject};

fn parse_args(args: &[&str]) -> CliOutcome {
    parse(std::iter::once("launcher").chain(args.iter().copied()))
}

fn ask(message: IpcMessage, json: bool) -> CliOutcome {
    CliOutcome::Ask(CliRequest { message, json })
}

fn remote(message: IpcMessage, json: bool) -> CliOutcome {
    CliOutcome::Remote(CliRequest { message, json })
}

fn exit_code(outcome: CliOutcome) -> i32 {
    match outcome {
        CliOutcome::Exit { code, .. } => code,
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn no_arguments_start_the_app_visibly_with_the_default_log_level() {
    assert_eq!(
        parse_args(&[]),
        CliOutcome::Run {
            hidden: false,
            log_level: None
        }
    );
}

#[test]
fn hidden_starts_the_app_without_a_window() {
    assert_eq!(
        parse_args(&["--hidden"]),
        CliOutcome::Run {
            hidden: true,
            log_level: None
        }
    );
}

#[test]
fn every_log_level_is_accepted() {
    for (word, level) in [
        ("error", LogLevel::Error),
        ("warn", LogLevel::Warn),
        ("info", LogLevel::Info),
        ("debug", LogLevel::Debug),
        ("trace", LogLevel::Trace),
    ] {
        assert_eq!(
            parse_args(&["--log-level", word]),
            CliOutcome::Run {
                hidden: false,
                log_level: Some(level)
            },
            "{word}"
        );
    }
}

#[test]
fn hidden_and_log_level_combine() {
    assert_eq!(
        parse_args(&["--hidden", "--log-level", "debug"]),
        CliOutcome::Run {
            hidden: true,
            log_level: Some(LogLevel::Debug)
        }
    );
}

#[test]
fn an_unknown_log_level_is_a_usage_error() {
    assert_eq!(exit_code(parse_args(&["--log-level", "loud"])), 2);
}

#[test]
fn log_levels_map_to_log_filters() {
    for (level, filter) in [
        (LogLevel::Error, log::LevelFilter::Error),
        (LogLevel::Warn, log::LevelFilter::Warn),
        (LogLevel::Info, log::LevelFilter::Info),
        (LogLevel::Debug, log::LevelFilter::Debug),
        (LogLevel::Trace, log::LevelFilter::Trace),
    ] {
        assert_eq!(log::LevelFilter::from(level), filter, "{level:?}");
    }
    assert_eq!(LogLevel::default(), LogLevel::Info);
}

#[test]
fn version_prints_the_name_and_version_and_exits_cleanly() {
    match parse_args(&["--version"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 0);
            assert_eq!(
                message.trim_end(),
                format!("launcher {}", env!("CARGO_PKG_VERSION"))
            );
        }
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn help_exits_cleanly_and_lists_the_subcommands() {
    match parse_args(&["--help"]) {
        CliOutcome::Exit { message, code } => {
            assert_eq!(code, 0);
            for word in [
                "status", "show", "focus", "hide", "quit", "--hidden", "--json",
            ] {
                assert!(message.contains(word), "{word}");
            }
        }
        other => panic!("expected an exit, got {other:?}"),
    }
}

#[test]
fn an_unknown_subcommand_is_a_usage_error() {
    assert_eq!(exit_code(parse_args(&["frobnicate"])), 2);
}

#[test]
fn status_hide_and_quit_ask_the_running_app_only() {
    for (word, message) in [
        ("status", IpcMessage::Status),
        ("hide", IpcMessage::Hide),
        ("quit", IpcMessage::Quit),
    ] {
        assert_eq!(parse_args(&[word]), ask(message, false), "{word}");
    }
}

#[test]
fn show_and_focus_may_start_the_app() {
    assert_eq!(
        parse_args(&["show"]),
        remote(IpcMessage::Show { focus: false }, false)
    );
    assert_eq!(
        parse_args(&["focus"]),
        remote(IpcMessage::Activate { hidden: false }, false)
    );
}

#[test]
fn json_is_accepted_before_or_after_the_subcommand() {
    for args in [&["status", "--json"][..], &["--json", "status"]] {
        assert_eq!(parse_args(args), ask(IpcMessage::Status, true), "{args:?}");
    }
    assert_eq!(
        parse_args(&["focus", "--json"]),
        remote(IpcMessage::Activate { hidden: false }, true)
    );
}

#[test]
fn status_renders_version_and_window_state() {
    for (visible, text) in [
        (true, "Launcher 0.1.0\nwindow: visible"),
        (false, "Launcher 0.1.0\nwindow: hidden"),
    ] {
        let reply = serde_json::json!({ "version": "0.1.0", "windowVisible": visible });
        assert_eq!(
            reply::render(&IpcMessage::Status, &reply),
            Ok(Some(text.to_string())),
            "{visible}"
        );
    }
}

#[test]
fn subcommands_without_output_render_nothing() {
    for message in [
        IpcMessage::Show { focus: false },
        IpcMessage::Activate { hidden: false },
        IpcMessage::Hide,
        IpcMessage::Quit,
    ] {
        assert_eq!(
            reply::render(&message, &serde_json::Value::Null),
            Ok(None),
            "{message:?}"
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn dev_seed_with_yes_asks_the_running_app_to_replace_the_data() {
    assert_eq!(
        parse_args(&["dev", "seed", "--yes"]),
        ask(IpcMessage::DevSeed { yes: true }, false)
    );
    assert_eq!(
        parse_args(&["dev", "seed", "--yes", "--json"]),
        ask(IpcMessage::DevSeed { yes: true }, true)
    );
}

#[cfg(debug_assertions)]
#[test]
fn dev_seed_without_yes_still_parses_and_leaves_the_refusal_to_the_app() {
    assert_eq!(
        parse_args(&["dev", "seed"]),
        ask(IpcMessage::DevSeed { yes: false }, false)
    );
}

#[cfg(debug_assertions)]
#[test]
fn dev_without_a_known_tool_is_a_usage_error() {
    for args in [
        &["dev"][..],
        &["dev", "frobnicate"],
        &["dev", "seed", "--force"],
    ] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
    }
}

fn cwd() -> std::path::PathBuf {
    std::env::current_dir().unwrap().canonicalize().unwrap()
}

fn home() -> std::path::PathBuf {
    std::env::home_dir().expect("a home folder")
}

fn text(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn project_subcommands_map_to_their_messages() {
    let cases: [(&[&str], IpcMessage); 13] = [
        (&["list"], IpcMessage::List),
        (&["commands"], IpcMessage::Commands),
        (&["palette"], IpcMessage::Palette),
        (&["settings"], IpcMessage::Settings),
        (
            &["open", "acme"],
            IpcMessage::Open {
                project: "acme".into(),
            },
        ),
        (
            &["describe", "acme"],
            IpcMessage::Describe {
                project: "acme".into(),
            },
        ),
        (
            &["launch", "acme", "editor"],
            IpcMessage::Launch {
                project: "acme".into(),
                name: "editor".into(),
            },
        ),
        (
            &["run", "acme", "build"],
            IpcMessage::Run {
                project: "acme".into(),
                name: "build".into(),
            },
        ),
        (
            &["terminal", "acme"],
            IpcMessage::Terminal {
                project: "acme".into(),
            },
        ),
        (
            &["finder", "acme"],
            IpcMessage::Finder {
                project: "acme".into(),
            },
        ),
        (
            &["config", "acme"],
            IpcMessage::Config {
                project: "acme".into(),
            },
        ),
        (
            &["db", "acme"],
            IpcMessage::Db {
                project: "acme".into(),
            },
        ),
        (
            &["command", "app:palette"],
            IpcMessage::Command {
                id: "app:palette".into(),
            },
        ),
    ];
    for (args, message) in cases {
        assert_eq!(parse_args(args), ask(message, false), "{args:?}");
    }
}

#[test]
fn describe_without_a_project_means_the_current_folder() {
    assert_eq!(
        parse_args(&["describe"]),
        ask(
            IpcMessage::Describe {
                project: text(&cwd())
            },
            false
        )
    );
}

#[test]
fn json_applies_to_the_project_subcommands() {
    assert_eq!(parse_args(&["list", "--json"]), ask(IpcMessage::List, true));
    assert_eq!(
        parse_args(&["run", "acme", "build", "--json"]),
        ask(
            IpcMessage::Run {
                project: "acme".into(),
                name: "build".into()
            },
            true
        )
    );
}

#[test]
fn import_takes_a_folder_and_an_optional_yes() {
    let folder = home().join("launcher-demo-prototype");
    for (args, yes) in [
        (&["import", "~/launcher-demo-prototype"][..], false),
        (&["import", "~/launcher-demo-prototype", "--yes"], true),
        (&["import", "--yes", "~/launcher-demo-prototype"], true),
    ] {
        assert_eq!(
            parse_args(args),
            ask(
                IpcMessage::Import {
                    folder: text(&folder),
                    yes
                },
                false
            ),
            "{args:?}"
        );
    }
}

#[test]
fn an_import_folder_is_always_made_absolute_even_as_a_bare_name() {
    let name = "launcher-demo-prototype-data";
    let expected = text(&std::env::current_dir().unwrap().join(name));
    assert_eq!(
        parse_args(&["import", name]),
        ask(
            IpcMessage::Import {
                folder: expected,
                yes: false
            },
            false
        )
    );
}

#[test]
fn a_project_given_as_a_folder_becomes_an_absolute_path() {
    let missing = "launcher-demo-missing-folder";
    for (spec, expected) in [
        (".".to_string(), text(&cwd())),
        (
            "..".to_string(),
            text(&cwd().parent().unwrap().to_path_buf()),
        ),
        (format!("~/{missing}"), text(&home().join(missing))),
        (
            format!("{missing}/inner"),
            text(&std::env::current_dir().unwrap().join(missing).join("inner")),
        ),
    ] {
        assert_eq!(
            parse_args(&["open", &spec]),
            ask(IpcMessage::Open { project: expected }, false),
            "{spec}"
        );
    }
}

#[test]
fn an_absolute_folder_stays_as_it_is() {
    let missing = "/launcher-demo-missing-root/acme";
    assert_eq!(
        parse_args(&["finder", missing]),
        ask(
            IpcMessage::Finder {
                project: missing.into()
            },
            false
        )
    );
}

#[test]
fn a_plain_name_or_id_stays_as_typed() {
    for spec in ["acme", "Acme Shop", "3f2a-demo-id"] {
        assert_eq!(
            parse_args(&["open", spec]),
            ask(
                IpcMessage::Open {
                    project: spec.into()
                },
                false
            ),
            "{spec}"
        );
    }
}

#[test]
fn project_subcommands_without_their_arguments_are_usage_errors() {
    for args in [
        &["open"][..],
        &["launch", "acme"],
        &["run", "acme"],
        &["command"],
        &["import"],
    ] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
    }
}

fn listed(name: &str, tag: Option<&str>, path: &str, archived: bool) -> ListedProject {
    ListedProject {
        id: format!("id-{name}"),
        name: name.into(),
        path: path.into(),
        tag: tag.map(str::to_string),
        starred: false,
        archived,
    }
}

#[test]
fn the_list_prints_as_a_table_with_archived_marked_and_the_home_folder_as_a_tilde() {
    let blog = text(&home().join("blog"));
    let reply = serde_json::to_value(vec![
        listed("Acme", Some("Client"), "/srv/acme", false),
        listed("Blog", None, &blog, true),
    ])
    .unwrap();

    let expected = [
        format!("{:<15}  {:<6}  {}", "NAME", "TAG", "PATH"),
        format!("{:<15}  {:<6}  {}", "Acme", "Client", "/srv/acme"),
        format!("{:<15}  {:<6}  {}", "Blog (archived)", "", "~/blog"),
    ]
    .join("\n");
    assert_eq!(reply::render(&IpcMessage::List, &reply), Ok(Some(expected)));
}

#[test]
fn only_paths_inside_the_home_folder_print_with_a_tilde() {
    let home = text(&home());
    let reply = serde_json::to_value(vec![
        listed("Home", None, &home, false),
        listed("Inside", None, &format!("{home}/work/acme"), false),
        listed("Sibling", None, &format!("{home}y/work"), false),
    ])
    .unwrap();

    let expected = [
        format!("{:<7}  {:<3}  {}", "NAME", "TAG", "PATH"),
        format!("{:<7}  {:<3}  {}", "Home", "", "~"),
        format!("{:<7}  {:<3}  {}", "Inside", "", "~/work/acme"),
        format!("{:<7}  {:<3}  {}", "Sibling", "", format!("{home}y/work")),
    ]
    .join("\n");
    assert_eq!(reply::render(&IpcMessage::List, &reply), Ok(Some(expected)));
}

#[test]
fn an_empty_list_says_there_are_no_projects() {
    assert_eq!(
        reply::render(&IpcMessage::List, &serde_json::json!([])),
        Ok(Some("No projects yet.".to_string()))
    );
}

fn describe_message() -> IpcMessage {
    IpcMessage::Describe {
        project: "acme".into(),
    }
}

fn description() -> Description {
    Description {
        project: listed(
            "Acme Shop",
            Some("Client"),
            &text(&home().join("acme-shop")),
            false,
        ),
        url: Some("https://example.com".into()),
        repo_url: Some("https://example.com/acme.git".into()),
        file_error: None,
        apps: vec![
            ("Editor".into(), "Nova".into()),
            ("Docs".into(), "https://example.com/docs".into()),
        ],
        scripts: vec![("Build".into(), "make".into())],
    }
}

#[test]
fn describe_prints_the_details_then_apps_and_scripts_sections() {
    let reply = serde_json::to_value(description()).unwrap();

    let expected = [
        "Acme Shop",
        "~/acme-shop",
        "Tag: Client",
        "Website: https://example.com",
        "Repository: https://example.com/acme.git",
        "",
        "Apps:",
        "  Editor  Nova",
        "  Docs    https://example.com/docs",
        "",
        "Scripts:",
        "  Build  make",
    ]
    .join("\n");
    assert_eq!(
        reply::render(&describe_message(), &reply),
        Ok(Some(expected))
    );
}

#[test]
fn describe_shows_paths_inside_the_home_folder_in_commands_with_a_tilde() {
    let home = text(&home());
    let reply = serde_json::to_value(Description {
        apps: vec![("Shell".into(), format!("cd {home}/work"))],
        scripts: vec![
            (
                "Export".into(),
                format!("ddev export-db --file={home}/dumps/a.sql"),
            ),
            ("Sibling".into(), format!("ls {home}y/work")),
        ],
        ..description()
    })
    .unwrap();

    let rendered = reply::render(&describe_message(), &reply).unwrap().unwrap();

    let commands: Vec<&str> = rendered
        .lines()
        .filter(|line| line.starts_with("  "))
        .collect();
    assert_eq!(
        commands,
        [
            "  Shell  cd ~/work".to_string(),
            "  Export   ddev export-db --file=~/dumps/a.sql".to_string(),
            format!("  Sibling  ls {home}y/work"),
        ]
    );
}

#[test]
fn describe_of_a_project_with_a_broken_file_shows_the_problem_and_no_sections() {
    let reply = serde_json::to_value(Description {
        project: listed("Acme Shop", None, &text(&home().join("acme-shop")), false),
        url: None,
        repo_url: None,
        file_error: Some(".project-launcher.json is missing".into()),
        apps: vec![],
        scripts: vec![],
        ..description()
    })
    .unwrap();

    assert_eq!(
        reply::render(&describe_message(), &reply),
        Ok(Some(
            "Acme Shop\n~/acme-shop\nProblem: .project-launcher.json is missing".to_string()
        ))
    );
}

#[test]
fn commands_print_as_an_id_and_title_table() {
    let reply = serde_json::json!([
        { "id": "app:palette", "title": "Open Palette" },
        { "id": "project:open", "title": "Open" }
    ]);
    assert_eq!(
        reply::render(&IpcMessage::Commands, &reply),
        Ok(Some(
            "app:palette   Open Palette\nproject:open  Open".to_string()
        ))
    );
}

#[test]
fn commands_ignore_extra_fields_and_a_malformed_reply_is_an_error() {
    let reply =
        serde_json::json!([{ "id": "app:palette", "title": "Open Palette", "help": "Shows it" }]);
    assert_eq!(
        reply::render(&IpcMessage::Commands, &reply),
        Ok(Some("app:palette  Open Palette".to_string()))
    );

    let err = reply::render(
        &IpcMessage::Commands,
        &serde_json::json!([{ "id": "app:palette" }]),
    )
    .unwrap_err();
    assert!(
        err.starts_with("The running Launcher sent an unexpected reply: "),
        "{err}"
    );
}

#[test]
fn run_prints_the_script_output_and_nothing_when_it_has_none() {
    let message = IpcMessage::Run {
        project: "acme".into(),
        name: "build".into(),
    };
    for (reply, expected) in [
        (
            serde_json::json!({ "output": "built 3 files\n" }),
            Some("built 3 files\n".to_string()),
        ),
        (serde_json::json!({ "output": "" }), None),
        (serde_json::Value::Null, None),
    ] {
        assert_eq!(reply::render(&message, &reply), Ok(expected), "{reply}");
    }
}

#[test]
fn db_prints_a_note_while_the_dev_server_starts_and_nothing_otherwise() {
    let message = IpcMessage::Db {
        project: "acme".into(),
    };
    assert_eq!(
        reply::render(&message, &serde_json::json!("starting")),
        Ok(Some(
            "Starting the dev server first; DataBee opens once it runs.".to_string()
        ))
    );
    assert_eq!(reply::render(&message, &serde_json::Value::Null), Ok(None));
}

fn import_message(yes: bool) -> IpcMessage {
    IpcMessage::Import {
        folder: "/tmp/prototype".into(),
        yes,
    }
}

#[test]
fn an_import_plan_prints_what_it_adds_and_asks_for_yes() {
    let plan = Plan {
        added: vec!["/work/acme".into()],
        known: vec!["/work/blog".into()],
        files: vec!["/work/acme/.project-launcher.json".into()],
        skipped: vec![Skipped {
            path: "/work/gone".into(),
            reason: "the folder is gone".into(),
        }],
        settings: vec!["Git client: Fork".into()],
        tags: vec!["Client".into()],
    };
    let reply = serde_json::to_value(&plan).unwrap();

    let expected = [
        "Adds:",
        "  /work/acme",
        "Already in the list:",
        "  /work/blog",
        "Writes:",
        "  /work/acme/.project-launcher.json",
        "Settings:",
        "  Git client: Fork",
        "Tags:",
        "  Client",
        "Skipped:",
        "  /work/gone (the folder is gone)",
        "",
        "Nothing changed yet. Run again with --yes to import.",
    ]
    .join("\n");
    assert_eq!(
        reply::render(&import_message(false), &reply),
        Ok(Some(expected))
    );
}

#[test]
fn an_applied_import_prints_what_it_did_and_how_to_undo() {
    let plan = Plan {
        added: vec!["/work/acme".into()],
        files: vec!["/work/acme/.project-launcher.json".into()],
        ..Plan::default()
    };
    let reply = serde_json::to_value(&plan).unwrap();

    let expected = [
        "Added:",
        "  /work/acme",
        "Wrote:",
        "  /work/acme/.project-launcher.json",
        "",
        "Imported. Edit → Undo in Launcher takes it back.",
    ]
    .join("\n");
    assert_eq!(
        reply::render(&import_message(true), &reply),
        Ok(Some(expected))
    );
}

#[test]
fn an_empty_import_plan_says_there_is_nothing_to_import() {
    let reply = serde_json::to_value(Plan::default()).unwrap();
    for yes in [false, true] {
        assert_eq!(
            reply::render(&import_message(yes), &reply),
            Ok(Some("Nothing to import.".to_string())),
            "{yes}"
        );
    }
}

#[test]
fn project_subcommands_without_output_render_nothing() {
    for message in [
        IpcMessage::Open {
            project: "acme".into(),
        },
        IpcMessage::Launch {
            project: "acme".into(),
            name: "editor".into(),
        },
        IpcMessage::Terminal {
            project: "acme".into(),
        },
        IpcMessage::Finder {
            project: "acme".into(),
        },
        IpcMessage::Config {
            project: "acme".into(),
        },
        IpcMessage::Palette,
        IpcMessage::Settings,
        IpcMessage::Command {
            id: "app:palette".into(),
        },
    ] {
        assert_eq!(
            reply::render(&message, &serde_json::Value::Null),
            Ok(None),
            "{message:?}"
        );
    }
}

#[test]
fn an_unexpected_status_reply_is_an_error() {
    let err = reply::render(&IpcMessage::Status, &serde_json::json!({ "version": 3 })).unwrap_err();
    assert!(
        err.starts_with("The running Launcher sent an unexpected reply: "),
        "{err}"
    );
}
