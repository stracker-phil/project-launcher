//! The command line's parsing: which words become which request, and how replies print.
use super::*;

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
    for args in [&["dev"][..], &["dev", "frobnicate"], &["dev", "seed", "--force"]] {
        assert_eq!(exit_code(parse_args(args)), 2, "{args:?}");
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
