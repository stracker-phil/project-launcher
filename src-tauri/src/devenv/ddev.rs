//! DDEV: `ddev … --json-output` prints one JSON object per line, log lines first, then the result (or a fatal
//! error). `describe` gives a project's state, URL and database; `list` every project on this Mac.

use std::path::PathBuf;

use serde_json::Value;

use super::{Connection, Described, ServerState};

/// The result line of `--json-output`: the last line with `raw`, or a fatal error.
pub fn result_line(output: &str) -> Option<Value> {
    output.lines().rev().find_map(|line| {
        let json: Value = serde_json::from_str(line.trim()).ok()?;
        (json.get("raw").is_some() || json.get("level").and_then(Value::as_str) == Some("fatal"))
            .then_some(json)
    })
}

fn message(json: Option<&Value>) -> Option<String> {
    json?.get("msg")?.as_str().map(str::to_string)
}

/// A project's state from `ddev describe`.
pub fn described(output: &str) -> Described {
    let json = result_line(output);
    let Some(raw) = json.as_ref().and_then(|j| j.get("raw")) else {
        return Described {
            state: ServerState::Unknown,
            url: None,
            detail: message(json.as_ref()).or(Some("ddev describe returned no result".into())),
        };
    };
    let status = raw
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let state = match status {
        "running" => ServerState::Running,
        "paused" => ServerState::Paused,
        "stopped" | "exited" => ServerState::Stopped,
        _ => ServerState::Unknown,
    };
    Described {
        state,
        url: raw
            .get("primary_url")
            .and_then(Value::as_str)
            .map(str::to_string),
        detail: Some(status.to_string()).filter(|s| !s.is_empty()),
    }
}

/// How to reach the database from this Mac (`dbinfo` of `ddev describe`).
pub fn connection(output: &str) -> Result<Connection, String> {
    let json = result_line(output);
    let info = json.as_ref().and_then(|j| j.get("raw")?.get("dbinfo"));
    let port = info
        .and_then(|i| i.get("published_port"))
        .and_then(|p| p.as_u64().or_else(|| p.as_str()?.parse().ok()))
        .and_then(|p| u16::try_from(p).ok())
        .filter(|p| *p > 0);
    let (Some(info), Some(port)) = (info, port) else {
        return Err(message(json.as_ref()).unwrap_or_else(|| {
            "DDEV didn’t report a database port. Is the project running?".into()
        }));
    };
    let text = |key: &str, default: &str| {
        info.get(key)
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string()
    };
    Ok(Connection {
        host: "127.0.0.1".into(),
        port,
        user: text("username", "db"),
        password: text("password", "db"),
        database: text("dbname", "db"),
    })
}

/// The folders of the running projects in `ddev list`.
pub fn running_paths(output: &str) -> Vec<PathBuf> {
    let json = result_line(output);
    let Some(list) = json.as_ref().and_then(|j| j.get("raw")?.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter(|p| p.get("status").and_then(Value::as_str) == Some("running"))
        .filter_map(|p| p.get("approot")?.as_str().map(PathBuf::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn output(lines: &[Value]) -> String {
        lines
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn describe_output(raw: Value) -> String {
        output(&[
            json!({ "level": "info", "msg": "Checking" }),
            json!({ "raw": raw }),
        ])
    }

    #[test]
    fn the_result_line_is_the_last_one_with_raw() {
        let text = output(&[
            json!({ "level": "info", "msg": "Starting" }),
            json!({ "raw": { "status": "stopped" } }),
            json!({ "raw": { "status": "running" } }),
        ]);

        assert_eq!(
            result_line(&text),
            Some(json!({ "raw": { "status": "running" } }))
        );
    }

    #[test]
    fn a_fatal_line_is_a_result_too() {
        let text = output(&[
            json!({ "level": "info", "msg": "Starting" }),
            json!({ "level": "fatal", "msg": "Docker isn’t running" }),
        ]);

        assert_eq!(
            result_line(&text),
            Some(json!({ "level": "fatal", "msg": "Docker isn’t running" }))
        );
    }

    #[test]
    fn lines_that_are_not_json_are_ignored() {
        let text = format!(
            "{}\nWarning: a plain line\n  \n",
            json!({ "raw": { "status": "running" } })
        );

        assert_eq!(
            result_line(&text),
            Some(json!({ "raw": { "status": "running" } }))
        );
    }

    #[test]
    fn output_with_only_log_lines_has_no_result_line() {
        let text = output(&[
            json!({ "level": "info", "msg": "Starting" }),
            json!({ "level": "warning", "msg": "Slow" }),
        ]);

        assert_eq!(result_line(&text), None);
        assert_eq!(result_line(""), None);
    }

    #[test]
    fn a_described_project_has_its_state_url_and_status() {
        for (status, state) in [
            ("running", ServerState::Running),
            ("paused", ServerState::Paused),
            ("stopped", ServerState::Stopped),
            ("exited", ServerState::Stopped),
            ("starting", ServerState::Unknown),
        ] {
            assert_eq!(
                described(&describe_output(json!({
                    "status": status,
                    "primary_url": "https://demo-site.ddev.site",
                }))),
                Described {
                    state,
                    url: Some("https://demo-site.ddev.site".into()),
                    detail: Some(status.into()),
                },
                "{status}"
            );
        }
    }

    #[test]
    fn a_described_project_without_status_or_url_is_unknown_without_detail() {
        assert_eq!(
            described(&describe_output(json!({}))),
            Described {
                state: ServerState::Unknown,
                url: None,
                detail: None,
            }
        );
    }

    #[test]
    fn a_fatal_message_is_the_detail_of_an_unknown_state() {
        let text = output(&[json!({ "level": "fatal", "msg": "Could not find the project" })]);

        assert_eq!(
            described(&text),
            Described {
                state: ServerState::Unknown,
                url: None,
                detail: Some("Could not find the project".into()),
            }
        );
    }

    #[test]
    fn a_describe_without_a_result_says_so() {
        assert_eq!(
            described("not json at all"),
            Described {
                state: ServerState::Unknown,
                url: None,
                detail: Some("ddev describe returned no result".into()),
            }
        );
    }

    #[test]
    fn the_connection_reads_the_published_port_as_a_number_or_a_string() {
        for port in [json!(32768), json!("32768")] {
            let text = describe_output(json!({
                "dbinfo": {
                    "published_port": port,
                    "username": "demo",
                    "password": "secret",
                    "dbname": "shop",
                },
            }));

            assert_eq!(
                connection(&text),
                Ok(Connection {
                    host: "127.0.0.1".into(),
                    port: 32768,
                    user: "demo".into(),
                    password: "secret".into(),
                    database: "shop".into(),
                }),
                "{port}"
            );
        }
    }

    #[test]
    fn the_connection_defaults_to_ddevs_db_credentials() {
        let text = describe_output(json!({ "dbinfo": { "published_port": 49153 } }));

        assert_eq!(
            connection(&text),
            Ok(Connection {
                host: "127.0.0.1".into(),
                port: 49153,
                user: "db".into(),
                password: "db".into(),
                database: "db".into(),
            })
        );
    }

    #[test]
    fn a_connection_without_a_usable_port_explains_it() {
        let message = "DDEV didn’t report a database port. Is the project running?";
        for raw in [
            json!({ "status": "stopped" }),
            json!({ "dbinfo": {} }),
            json!({ "dbinfo": { "published_port": "" } }),
            json!({ "dbinfo": { "published_port": "abc" } }),
            json!({ "dbinfo": { "published_port": 0 } }),
            json!({ "dbinfo": { "published_port": 70000 } }),
        ] {
            assert_eq!(
                connection(&describe_output(raw.clone())),
                Err(message.to_string()),
                "{raw}"
            );
        }
        assert_eq!(connection(""), Err(message.to_string()));
    }

    #[test]
    fn a_fatal_message_is_the_error_of_a_connection() {
        let text = output(&[json!({ "level": "fatal", "msg": "Could not find the project" })]);

        assert_eq!(
            connection(&text),
            Err("Could not find the project".to_string())
        );
    }

    #[test]
    fn only_running_projects_are_listed_by_their_folders() {
        let text = describe_output(json!([
            { "name": "demo-site", "status": "running", "approot": "/demo/site" },
            { "name": "demo-shop", "status": "stopped", "approot": "/demo/shop" },
            { "name": "demo-blog", "status": "paused", "approot": "/demo/blog" },
            { "name": "demo-docs", "status": "running", "approot": "/demo/docs" },
            { "name": "demo-lost", "status": "running" },
        ]));

        assert_eq!(
            running_paths(&text),
            [PathBuf::from("/demo/site"), PathBuf::from("/demo/docs")]
        );
    }

    #[test]
    fn a_list_without_an_array_has_no_running_paths() {
        for text in [
            String::new(),
            describe_output(json!({ "status": "running" })),
            describe_output(Value::Null),
        ] {
            assert_eq!(running_paths(&text), Vec::<PathBuf>::new(), "{text}");
        }
    }
}
