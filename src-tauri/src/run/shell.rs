//! Shell command text: quoting, opening with an app or a CLI, and the scripts terminal sessions run.

use std::path::Path;

/// One argument for `zsh`, in single quotes.
pub fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Runs `app` on `target`: a CLI on the login PATH (`code`, `smerge`) is called directly, anything else is a
/// macOS app name for `open -a`. `app` may carry arguments of its own (`code --new-window`).
pub fn open_with(app: &str, target: &str) -> String {
    let app = app.trim();
    let bin = app.split_whitespace().next().unwrap_or(app);
    format!(
        "if command -v {bin} >/dev/null 2>&1; then {app} {target}; else open -a {app_name} {target}; fi",
        bin = quote(bin),
        target = quote(target),
        app_name = quote(app),
    )
}

/// The `.command` file a terminal opens: the project folder, its env, then `command` (if any), then a login
/// shell that stays open. With `session`, the command records where it runs while it runs.
pub fn terminal_script(
    dir: &Path,
    env: &[(String, String)],
    command: Option<&str>,
    session: Option<&Path>,
) -> String {
    let mut lines = vec![
        "#!/bin/zsh -l".to_string(),
        format!("cd {} || exit 1", quote(&dir.to_string_lossy())),
    ];
    lines.extend(
        env.iter()
            .map(|(name, value)| format!("export {name}={}", quote(value))),
    );
    lines.push("clear".into());
    match (command.filter(|c| !c.trim().is_empty()), session) {
        (Some(command), Some(session)) => {
            let file = quote(&session.to_string_lossy());
            lines.push(format!(
                r#"printf '{{"pid":%d,"tty":"%s"}}\n' $$ "$(tty)" > {file}"#
            ));
            lines.push(command.into());
            lines.push(format!("rm -f {file}"));
        }
        (Some(command), None) => lines.push(command.into()),
        (None, _) => {}
    }
    lines.push(r#"exec "${SHELL:-/bin/zsh}" -l"#.into());
    lines.join("\n") + "\n"
}

/// A name that is safe in a file name, at most 40 characters; "project" when nothing is left.
pub fn file_safe(name: &str) -> String {
    let mut safe = String::new();
    let mut dash = false;
    for c in name.chars() {
        if c.is_alphanumeric() || c == '.' || c == '_' {
            safe.push(c);
            dash = false;
        } else if !dash {
            safe.push('-');
            dash = true;
        }
    }
    let cut: String = safe.trim_start_matches('-').chars().take(40).collect();
    let safe = cut.trim_end_matches('-');
    if safe.is_empty() {
        "project".into()
    } else {
        safe.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn quote_wraps_in_single_quotes_and_escapes_inner_ones() {
        for (input, expected) in [
            ("plain", "'plain'"),
            ("two words", "'two words'"),
            ("it's", r"'it'\''s'"),
            ("", "''"),
        ] {
            assert_eq!(quote(input), expected, "{input}");
        }
    }

    #[test]
    fn open_with_calls_a_cli_directly_and_falls_back_to_open_a() {
        assert_eq!(
            open_with("code --new-window", "/work/demo app"),
            "if command -v 'code' >/dev/null 2>&1; then code --new-window '/work/demo app'; \
             else open -a 'code --new-window' '/work/demo app'; fi"
        );
    }

    #[test]
    fn open_with_checks_the_first_word_of_an_app_name_and_trims_it() {
        assert_eq!(
            open_with("  Sketchpad Pro ", "/work/it's"),
            "if command -v 'Sketchpad' >/dev/null 2>&1; then Sketchpad Pro '/work/it'\\''s'; \
             else open -a 'Sketchpad Pro' '/work/it'\\''s'; fi"
        );
    }

    #[test]
    fn a_terminal_script_enters_the_folder_exports_env_and_ends_in_a_login_shell() {
        let script = terminal_script(
            Path::new("/work/demo"),
            &env(&[("STAGE", "demo"), ("NOTE", "it's")]),
            None,
            None,
        );
        assert_eq!(
            script,
            "#!/bin/zsh -l\n\
             cd '/work/demo' || exit 1\n\
             export STAGE='demo'\n\
             export NOTE='it'\\''s'\n\
             clear\n\
             exec \"${SHELL:-/bin/zsh}\" -l\n"
        );
    }

    #[test]
    fn a_terminal_script_runs_the_command_before_the_shell() {
        let script = terminal_script(Path::new("/work/demo"), &[], Some("npm run dev"), None);
        assert_eq!(
            script,
            "#!/bin/zsh -l\n\
             cd '/work/demo' || exit 1\n\
             clear\n\
             npm run dev\n\
             exec \"${SHELL:-/bin/zsh}\" -l\n"
        );
    }

    #[test]
    fn a_terminal_script_with_a_session_records_pid_and_tty_while_the_command_runs() {
        let script = terminal_script(
            Path::new("/work/demo"),
            &[],
            Some("npm run dev"),
            Some(Path::new("/tmp/sessions/a b.json")),
        );
        let expected = [
            "#!/bin/zsh -l",
            "cd '/work/demo' || exit 1",
            "clear",
            r#"printf '{"pid":%d,"tty":"%s"}\n' $$ "$(tty)" > '/tmp/sessions/a b.json'"#,
            "npm run dev",
            "rm -f '/tmp/sessions/a b.json'",
            r#"exec "${SHELL:-/bin/zsh}" -l"#,
        ]
        .join("\n")
            + "\n";
        assert_eq!(script, expected);
    }

    #[test]
    fn an_empty_or_blank_command_adds_no_command_and_no_session_lines() {
        let bare = terminal_script(Path::new("/work/demo"), &[], None, None);
        for command in [None, Some(""), Some("  \n ")] {
            let script = terminal_script(
                Path::new("/work/demo"),
                &[],
                command,
                Some(Path::new("/tmp/s.json")),
            );
            assert_eq!(script, bare, "{command:?}");
        }
    }

    #[test]
    fn file_safe_collapses_unsafe_runs_into_single_dashes_and_trims_them() {
        for (input, expected) in [
            ("Demo App", "Demo-App"),
            ("  --a/b\\c  ", "a-b-c"),
            ("a  //  b", "a-b"),
            ("v1.2_final", "v1.2_final"),
            ("Café Né", "Café-Né"),
        ] {
            assert_eq!(file_safe(input), expected, "{input:?}");
        }
    }

    #[test]
    fn file_safe_is_project_when_nothing_is_left() {
        for input in ["", "   ", "///", "--"] {
            assert_eq!(file_safe(input), "project", "{input:?}");
        }
    }

    #[test]
    fn file_safe_stops_at_forty_characters() {
        assert_eq!(file_safe(&"a".repeat(60)), "a".repeat(40));
    }

    #[test]
    fn file_safe_does_not_end_in_a_dash_after_cutting() {
        assert_eq!(file_safe(&format!("{} b", "a".repeat(39))), "a".repeat(39));
    }
}
