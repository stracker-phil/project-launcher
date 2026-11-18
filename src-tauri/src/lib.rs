//! Launcher's library: the modules and `run`, which reads the command line, hands off to a running
//! Launcher or starts the app.

mod app;
pub mod automation;
mod autostart;
pub mod bus;
pub mod claude_manager;
pub mod cli;
mod commands;
pub mod data;
#[cfg(debug_assertions)]
mod dev;
pub mod devenv;
pub mod error;
pub mod events;
mod folders;
pub mod global_shortcut;
mod handlers;
mod instance;
pub mod ipc;
pub mod launch;
pub mod library;
pub mod links;
pub mod lock;
pub mod menu;
pub mod projects;
mod remote;
pub mod run;
pub mod state;
pub mod stats;
pub mod status;
pub mod store;
#[cfg(any(test, debug_assertions, feature = "test-util"))]
pub mod test_support;
mod tray;
pub mod view_state;
pub mod window;

use std::path::PathBuf;

use crate::cli::{CliOutcome, CliRequest};
use crate::ipc::IpcMessage;
use crate::state::AppState;
use crate::store::create_private_dir;

/// The bundle identifier and config folder name; the debug build's own keeps its data apart (ADR 0022).
const IDENTIFIER: &str = if cfg!(debug_assertions) {
    "com.philippstracker.launcher.debug"
} else {
    "com.philippstracker.launcher"
};
/// Debug builds go by their own name in the menu bar and the Dock.
pub const APP_NAME: &str = if cfg!(debug_assertions) {
    "Launcher (Debug)"
} else {
    "Launcher"
};
const SOCKET_NAME: &str = "launcher.sock";
/// Debug builds claim their own scheme (`tauri.debug.conf.json`), so links reach the installed app.
pub const LINK_SCHEME: &str = if cfg!(debug_assertions) {
    "launcher-debug"
} else {
    "launcher"
};

/// Same directory Tauri resolves as `app_config_dir`, but available before the app starts.
pub(crate) fn config_dir() -> PathBuf {
    let Some(home) = std::env::home_dir() else {
        fail("Cannot locate the home directory for Launcher's settings");
    };
    home.join("Library/Application Support").join(IDENTIFIER)
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// A launch from Finder, Spotlight, `open` or at login starts in the menu bar (ADR 0044); `launcher` typed in a
/// terminal shows the window unless asked for `--hidden`.
pub fn starts_hidden(asked_hidden: bool, typed_in_a_terminal: bool) -> bool {
    asked_hidden || !typed_in_a_terminal
}

/// Whether stdin is a terminal: LaunchServices starts apps without one.
fn typed_in_a_terminal() -> bool {
    // SAFETY: isatty only reads the state of file descriptor 0.
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
}

pub fn run() {
    let config_dir = config_dir();
    if let Err(e) = create_private_dir(&config_dir) {
        eprintln!("Cannot create {}: {e}", config_dir.display());
    }
    let socket = config_dir.join(SOCKET_NAME);

    let (message, log_level, hidden) = match cli::parse(std::env::args_os()) {
        CliOutcome::Exit { message, code } => {
            if code == 0 {
                print!("{message}");
            } else {
                eprint!("{message}");
            }
            std::process::exit(code);
        }
        CliOutcome::Run { hidden, log_level } => {
            let hidden = starts_hidden(hidden, typed_in_a_terminal());
            (IpcMessage::Activate { hidden }, log_level, hidden)
        }
        CliOutcome::Ask(CliRequest { message, json }) => {
            match instance::ask(&socket, &message, json) {
                Some(code) => std::process::exit(code),
                None => fail("Launcher isn't running"),
            }
        }
        CliOutcome::Remote(CliRequest { message, json }) => {
            if let Some(code) = instance::ask(&socket, &message, json) {
                std::process::exit(code);
            }
            (message, None, false)
        }
    };

    let _instance_lock = instance::forward_or_lock(&config_dir, &socket, &message);
    let state = AppState::open(&config_dir)
        .unwrap_or_else(|e| fail(&format!("Launcher can't open its data: {e}")));
    app::start(
        state,
        socket,
        message,
        log_level.unwrap_or_default(),
        hidden,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configs() -> (serde_json::Value, serde_json::Value) {
        let release = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let debug = serde_json::from_str(include_str!("../tauri.debug.conf.json")).unwrap();
        (release, debug)
    }

    fn this_build() -> serde_json::Value {
        let (release, debug) = configs();
        if cfg!(debug_assertions) {
            debug
        } else {
            release
        }
    }

    #[test]
    fn version_is_the_same_in_cargo_tauri_config_and_package_json() {
        let (release, _) = configs();
        let package: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../package.json"))
                .unwrap(),
        )
        .unwrap();
        let cargo = env!("CARGO_PKG_VERSION");

        assert_eq!(
            release["version"],
            serde_json::json!(cargo),
            "tauri.conf.json"
        );
        assert_eq!(package["version"], serde_json::json!(cargo), "package.json");
        let parts: Vec<&str> = cargo.split('.').collect();
        assert!(
            parts.len() == 3
                && parts
                    .iter()
                    .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())),
            "{cargo}"
        );
    }

    #[test]
    fn no_http_client_or_server_among_the_direct_dependencies() {
        let manifest =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
        let mut in_dependencies = false;
        let mut names = Vec::new();
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                in_dependencies = line == "[dependencies]";
            } else if in_dependencies
                && !line.starts_with('#')
                && let Some((key, _)) = line.split_once('=')
            {
                names.push(key.trim().trim_matches('"').to_string());
            }
        }
        assert!(names.contains(&"tauri".to_string()), "{names:?}");

        let forbidden = [
            "reqwest",
            "hyper",
            "ureq",
            "isahc",
            "surf",
            "attohttpc",
            "tiny_http",
            "axum",
            "actix-web",
            "warp",
            "tauri-plugin-http",
            "tauri-plugin-updater",
        ];
        let found: Vec<&String> = names
            .iter()
            .filter(|name| forbidden.contains(&name.as_str()))
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn only_a_terminal_launch_without_the_hidden_flag_shows_the_window() {
        for (asked_hidden, typed_in_a_terminal, expected) in [
            (false, true, false),
            (true, true, true),
            (false, false, true),
            (true, false, true),
        ] {
            assert_eq!(
                starts_hidden(asked_hidden, typed_in_a_terminal),
                expected,
                "asked_hidden={asked_hidden}, typed_in_a_terminal={typed_in_a_terminal}"
            );
        }
    }

    #[test]
    fn identifier_matches_the_tauri_config() {
        assert_eq!(this_build()["identifier"], serde_json::json!(IDENTIFIER));
    }

    #[test]
    fn app_name_matches_the_product_name_in_the_tauri_config() {
        assert_eq!(this_build()["productName"], serde_json::json!(APP_NAME));
    }

    #[test]
    fn link_scheme_matches_the_deep_link_scheme_in_the_tauri_config() {
        assert_eq!(
            this_build()["plugins"]["deep-link"]["desktop"]["schemes"],
            serde_json::json!([LINK_SCHEME])
        );
    }

    #[test]
    fn the_debug_build_differs_from_the_release_build_in_name_identifier_and_scheme() {
        let (release, debug) = configs();
        assert_eq!(release["productName"], "Launcher");
        assert_eq!(debug["productName"], "Launcher (Debug)");
        assert_eq!(
            debug["identifier"],
            serde_json::json!(format!("{}.debug", release["identifier"].as_str().unwrap()))
        );
        let schemes = |config: &serde_json::Value| {
            config["plugins"]["deep-link"]["desktop"]["schemes"].clone()
        };
        assert_eq!(schemes(&release), serde_json::json!(["launcher"]));
        assert_eq!(schemes(&debug), serde_json::json!(["launcher-debug"]));
    }
}
