//! What each built-in opens in a project, from its `meta` and the settings, or why it can't run there.

use super::model::{Builtin, RunsAs};
use super::resolve::{BuiltinApp, Context};
use serde_json::Value;

/// The `meta` values the built-ins use.
pub struct Links {
    pub url: Option<String>,
    pub repo_url: Option<String>,
    pub editor: Option<String>,
}

#[derive(Default)]
pub struct Target {
    pub app: Option<String>,
    pub url: Option<String>,
    pub command: Option<String>,
}

impl Target {
    fn app(app: String) -> Self {
        Target {
            app: Some(app),
            ..Target::default()
        }
    }

    fn url(url: String) -> Self {
        Target {
            url: Some(url),
            ..Target::default()
        }
    }

    /// An empty command opens a shell in the project folder.
    fn command(command: &str) -> Self {
        Target {
            command: Some(command.into()),
            ..Target::default()
        }
    }
}

/// An app wins over a URL; with neither, it is a terminal command.
pub fn runs_as(has_app: bool, has_url: bool) -> RunsAs {
    match (has_app, has_url) {
        (true, _) => RunsAs::App,
        (false, true) => RunsAs::Url,
        _ => RunsAs::Terminal,
    }
}

pub fn target(preset: Builtin, cx: &Context, links: &Links) -> Result<Target, String> {
    let non_blank = |value: &str| {
        Some(value.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    match preset {
        Builtin::Editor => links
            .editor
            .clone()
            .or_else(|| non_blank(&cx.settings.default_editor))
            .map(Target::app)
            .ok_or_else(|| "Choose a default editor in Settings".into()),
        Builtin::Terminal => Ok(Target::command("")),
        Builtin::Git if !cx.is_git_repo => Err("Not a git repository".into()),
        Builtin::Git => non_blank(&cx.settings.git_client)
            .map(Target::app)
            .ok_or_else(|| "Choose a git client in Settings".into()),
        Builtin::Browser => links
            .url
            .clone()
            .map(Target::url)
            .ok_or_else(|| "Add a website URL first".into()),
        Builtin::RepoBrowser => links
            .repo_url
            .clone()
            .map(Target::url)
            .ok_or_else(|| "Add a repository URL first".into()),
        Builtin::Claude => Ok(Target::command("claude")),
    }
}

/// Every built-in, with whether `apps` lists it and why it can't run here.
pub fn builtin_apps(apps: &[Value], cx: &Context, links: &Links) -> Vec<BuiltinApp> {
    Builtin::ALL
        .into_iter()
        .map(|preset| BuiltinApp {
            preset,
            label: preset.label().into(),
            icon: preset.icon().into(),
            enabled: apps.iter().any(|e| Builtin::of(e) == Some(preset)),
            unavailable_reason: target(preset, cx, links).err(),
        })
        .collect()
}
