//! The Claude Code setup of a folder with a `.claude` directory: which profile a session there uses.

use std::path::Path;

use serde::Serialize;
use ts_rs::TS;

use super::resolve::EnvVar;

/// Claude Code's profile in the home folder when `CLAUDE_CONFIG_DIR` isn't set.
const DEFAULT_PROFILE: &str = ".claude";

#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSetup {
    /// The profile's config folder: `CLAUDE_CONFIG_DIR` from the env, else `~/.claude`.
    pub profile: String,
}

fn default_profile(home: Option<&Path>) -> String {
    match home {
        Some(home) => home.join(DEFAULT_PROFILE).to_string_lossy().into_owned(),
        None => format!("~/{DEFAULT_PROFILE}"),
    }
}

/// The setup when the folder has a `.claude` directory.
pub fn setup(uses_claude: bool, env: &[EnvVar], home: Option<&Path>) -> Option<ClaudeSetup> {
    uses_claude.then(|| ClaudeSetup {
        profile: env
            .iter()
            .find(|v| v.name == "CLAUDE_CONFIG_DIR")
            .map_or_else(|| default_profile(home), |v| v.value.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, value: &str) -> EnvVar {
        EnvVar {
            name: name.into(),
            value: value.into(),
        }
    }

    fn profile(env: &[EnvVar], home: Option<&str>) -> Option<String> {
        setup(true, env, home.map(Path::new)).map(|claude| claude.profile)
    }

    #[test]
    fn there_is_no_setup_without_a_claude_directory() {
        let env = [var("CLAUDE_CONFIG_DIR", "/srv/profiles/demo")];

        assert_eq!(setup(false, &env, Some(Path::new("/Users/demo"))), None);
        assert_eq!(setup(false, &[], None), None);
    }

    #[test]
    fn the_profile_defaults_to_the_claude_folder_in_the_home_folder() {
        assert_eq!(
            setup(true, &[], Some(Path::new("/Users/demo"))),
            Some(ClaudeSetup {
                profile: "/Users/demo/.claude".into(),
            })
        );
    }

    #[test]
    fn without_a_home_folder_the_default_profile_is_the_tilde_one() {
        assert_eq!(profile(&[], None), Some("~/.claude".to_string()));
    }

    #[test]
    fn config_dir_from_the_env_wins_over_the_default_profile() {
        let env = [var("CLAUDE_CONFIG_DIR", "/srv/profiles/demo")];
        for home in [Some("/Users/demo"), None] {
            assert_eq!(
                profile(&env, home),
                Some("/srv/profiles/demo".to_string()),
                "{home:?}"
            );
        }
    }

    #[test]
    fn other_env_vars_do_not_change_the_profile() {
        let env = [var("APP_ENV", "local"), var("CLAUDE_CONFIG", "/elsewhere")];

        assert_eq!(
            profile(&env, Some("/Users/demo")),
            Some("/Users/demo/.claude".to_string())
        );
    }

    #[test]
    fn the_setup_serializes_its_profile_for_the_page() {
        let value = serde_json::to_value(setup(true, &[], None).unwrap()).unwrap();

        assert_eq!(value, serde_json::json!({ "profile": "~/.claude" }));
    }
}
