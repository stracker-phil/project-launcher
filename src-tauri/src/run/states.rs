//! Project states: each state provider runs once, and each state maps its provider's output (or a field of
//! its JSON, `name:.path.to.field`) onto the values the file defines.

use std::collections::HashMap;
use std::path::Path;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

use super::system::{Job, Output, System};
use crate::projects::config::resolve::{StateDef, StateProvider};

const PROVIDER_TIMEOUT: Duration = Duration::from_secs(5);

/// A state as the detail shows it; `color` only when the value is one the file defines.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct StateDisplay {
    pub label: String,
    pub value: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// `keys`: the defined values that match now (what `hiddenStates` names).
#[derive(Debug, Default, PartialEq)]
pub struct States {
    pub keys: Vec<String>,
    pub display: Vec<StateDisplay>,
}

/// A provider's output: its text when it succeeded, and that text as JSON when it is.
struct Printed {
    raw: String,
    json: Option<Value>,
}

impl From<Output> for Printed {
    fn from(output: Output) -> Self {
        let raw = if output.ok() {
            output.stdout.trim().to_string()
        } else {
            String::new()
        };
        let json = serde_json::from_str(&raw).ok();
        Printed { raw, json }
    }
}

/// The text a state reads: its provider's output, or a field of its JSON.
fn reading(state: &StateDef, printed: &HashMap<String, Printed>) -> Option<String> {
    let (name, path) = split_source(&state.source);
    let output = printed.get(name)?;
    match path {
        Some(path) => output.json.as_ref().and_then(|json| field(json, path)),
        None => Some(&output.raw).filter(|raw| !raw.is_empty()).cloned(),
    }
}

/// The provider a source reads and the JSON path, if any.
fn split_source(source: &str) -> (&str, Option<&str>) {
    match source.split_once(':') {
        Some((name, path)) if !name.is_empty() && path.starts_with('.') => (name, Some(path)),
        _ => (source, None),
    }
}

/// The field at `.a.b` as text; `None` when it is missing or null.
fn field(json: &Value, path: &str) -> Option<String> {
    let value = path
        .split('.')
        .filter(|part| !part.is_empty())
        .try_fold(json, |value, part| value.get(part))?;
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

/// Runs the providers the states read, each once and all at the same time.
fn run_providers(
    system: &dyn System,
    dir: &Path,
    env: &[(String, String)],
    providers: &[StateProvider],
    states: &[StateDef],
) -> HashMap<String, Printed> {
    let needed: Vec<&StateProvider> = providers
        .iter()
        .filter(|p| states.iter().any(|s| split_source(&s.source).0 == p.name))
        .collect();
    thread::scope(|scope| {
        let runs: Vec<_> = needed
            .iter()
            .map(|provider| {
                let job = Job {
                    command: provider.command.clone(),
                    cwd: Some(dir.to_path_buf()),
                    env: env.to_vec(),
                };
                (
                    provider.name.clone(),
                    scope.spawn(move || system.run(&job, PROVIDER_TIMEOUT)),
                )
            })
            .collect();
        runs.into_iter()
            .filter_map(|(name, run)| Some((name, Printed::from(run.join().ok()?))))
            .collect()
    })
}

/// The project's states now; a state whose provider printed nothing (or failed) is left out.
pub fn resolve(
    system: &dyn System,
    dir: &Path,
    env: &[(String, String)],
    providers: &[StateProvider],
    states: &[StateDef],
) -> States {
    let printed = run_providers(system, dir, env, providers, states);
    let mut result = States::default();
    for state in states {
        let Some(raw) = reading(state, &printed) else {
            continue;
        };
        match state.values.iter().find(|v| v.value == raw) {
            Some(value) => {
                result.keys.push(value.key.clone());
                result.display.push(StateDisplay {
                    label: state.label.clone(),
                    value: value.label.clone(),
                    color: value.color.clone(),
                });
            }
            None => result.display.push(StateDisplay {
                label: state.label.clone(),
                value: raw,
                color: None,
            }),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::projects::config::resolve::StateValue;
    use crate::run::system::Recorder;

    const DIR: &str = "/work/northwind";

    fn provider(name: &str, command: &str) -> StateProvider {
        StateProvider {
            name: name.into(),
            command: command.into(),
        }
    }

    fn value(key: &str, value: &str, label: &str, color: Option<&str>) -> StateValue {
        StateValue {
            key: key.into(),
            value: value.into(),
            label: label.into(),
            color: color.map(Into::into),
        }
    }

    fn state(name: &str, source: &str, label: &str, values: Vec<StateValue>) -> StateDef {
        StateDef {
            name: name.into(),
            source: source.into(),
            label: label.into(),
            values,
        }
    }

    fn display(label: &str, value: &str, color: Option<&str>) -> StateDisplay {
        StateDisplay {
            label: label.into(),
            value: value.into(),
            color: color.map(Into::into),
        }
    }

    fn resolved(system: &Recorder, providers: &[StateProvider], states: &[StateDef]) -> States {
        resolve(system, Path::new(DIR), &[], providers, states)
    }

    fn site_values() -> Vec<StateValue> {
        vec![
            value("live", "up", "Online", Some("green")),
            value("down", "down", "Offline", None),
        ]
    }

    #[test]
    fn plain_output_maps_to_the_defined_value_with_its_key_label_and_color() {
        let system = Recorder::default();
        system.reply("check-site", "up\n");

        let states = resolved(
            &system,
            &[provider("web", "check-site")],
            &[state("site", "web", "Site", site_values())],
        );

        assert_eq!(
            states,
            States {
                keys: vec!["live".into()],
                display: vec![display("Site", "Online", Some("green"))],
            }
        );
    }

    #[test]
    fn a_value_without_a_color_shows_without_one() {
        let system = Recorder::default();
        system.reply("check-site", "down");

        let states = resolved(
            &system,
            &[provider("web", "check-site")],
            &[state("site", "web", "Site", site_values())],
        );

        assert_eq!(states.keys, ["down"]);
        assert_eq!(states.display, [display("Site", "Offline", None)]);
    }

    #[test]
    fn an_output_the_file_does_not_define_shows_raw_without_color_or_key() {
        let system = Recorder::default();
        system.reply("check-site", "degraded\n");

        let states = resolved(
            &system,
            &[provider("web", "check-site")],
            &[state("site", "web", "Site", site_values())],
        );

        assert_eq!(
            states,
            States {
                keys: vec![],
                display: vec![display("Site", "degraded", None)],
            }
        );
    }

    #[test]
    fn a_json_field_is_read_by_its_path_and_numbers_and_booleans_become_text() {
        let system = Recorder::default();
        system.reply(
            "ci-status",
            r#"{"build":{"number":42,"branch":"main","passed":false,"ratio":1.5}}"#,
        );
        let states = [
            state("number", "ci:.build.number", "Build", vec![]),
            state(
                "branch",
                "ci:.build.branch",
                "Branch",
                vec![value("trunk", "main", "Trunk", Some("blue"))],
            ),
            state("passed", "ci:.build.passed", "Passed", vec![]),
            state("ratio", "ci:.build.ratio", "Ratio", vec![]),
        ];

        let resolved = resolved(&system, &[provider("ci", "ci-status")], &states);

        assert_eq!(resolved.keys, ["trunk"]);
        assert_eq!(
            resolved.display,
            [
                display("Build", "42", None),
                display("Branch", "Trunk", Some("blue")),
                display("Passed", "false", None),
                display("Ratio", "1.5", None),
            ]
        );
    }

    #[test]
    fn a_state_whose_output_cannot_be_read_is_left_out() {
        let cases = [
            ("missing field", "ci:.nope", r#"{"a":1}"#),
            ("null field", "ci:.a", r#"{"a":null}"#),
            ("path into a non-object", "ci:.a.b", r#"{"a":1}"#),
            ("path on text", "ci:.a", "not json"),
            ("empty output", "ci", ""),
            ("blank output", "ci", " \n"),
            ("unknown provider", "ghost", "up"),
        ];
        for (name, source, output) in cases {
            let system = Recorder::default();
            system.reply("ci-status", output);

            let states = resolved(
                &system,
                &[provider("ci", "ci-status")],
                &[state("s", source, "S", site_values())],
            );

            assert_eq!(states, States::default(), "{name}");
        }
    }

    #[test]
    fn states_sharing_a_provider_each_resolve_and_keep_file_order() {
        let system = Recorder::default();
        system.reply("docker-info", r#"{"web":"up","db":"down"}"#);
        let states = [
            state("web", "docker:.web", "Web", site_values()),
            state("db", "docker:.db", "Database", site_values()),
        ];

        let resolved = resolved(&system, &[provider("docker", "docker-info")], &states);

        assert_eq!(resolved.keys, ["live", "down"]);
        assert_eq!(
            resolved.display,
            [
                display("Web", "Online", Some("green")),
                display("Database", "Offline", None),
            ]
        );
    }

    #[test]
    fn without_states_the_result_is_empty_even_when_providers_would_answer() {
        let system = Recorder::default();
        system.reply("check-site", "up");

        let states = resolved(&system, &[provider("web", "check-site")], &[]);

        assert_eq!(states, States::default());
    }

    #[test]
    fn a_state_display_serializes_camel_case_and_omits_a_missing_color() {
        assert_eq!(
            serde_json::to_value(display("Site", "Online", Some("green"))).unwrap(),
            json!({ "label": "Site", "value": "Online", "color": "green" })
        );
        assert_eq!(
            serde_json::to_value(display("Site", "raw", None)).unwrap(),
            json!({ "label": "Site", "value": "raw" })
        );
    }
}
