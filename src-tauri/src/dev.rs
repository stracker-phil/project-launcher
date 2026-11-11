//! Debug-build conveniences for driving the app from scripts/dev without disturbing the owner: window
//! placement, background launches, running JavaScript (ADR 0023) and the demo seed (ADR 0022). Only compiled
//! into debug builds (`#[cfg(debug_assertions)]` on the module and its call sites).

use std::path::Path;

use crate::data::{PROJECTS_FILE, SETTINGS_FILE};
use crate::error::{AppError, AppResult};
use crate::store::json_file::JsonFile;
use crate::test_support::demo;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Writes the demo seed when the data folder has no projects list yet: the debug build's first launch.
pub fn seed_first_run(config_dir: &Path) -> AppResult<()> {
    if config_dir.join(PROJECTS_FILE).exists() {
        return Ok(());
    }
    log::info!("First launch of the debug build: writing the demo seed");
    let seed = demo::write(config_dir, now_ms())?;
    JsonFile::new(config_dir.join(SETTINGS_FILE)).save(&seed.settings)?;
    JsonFile::new(config_dir.join(PROJECTS_FILE)).save(&seed.registry)
}

/// `launcher dev seed --yes`: replaces the debug data with the demo seed; refused without `yes`.
pub fn seed(app: &tauri::AppHandle, yes: bool) -> AppResult<()> {
    use tauri::Manager;
    if !yes {
        return Err(AppError::InvalidInput(
            "This replaces the debug build's projects and settings with the demo seed; pass --yes".into(),
        ));
    }
    let seed = demo::write(&crate::config_dir(), now_ms())?;
    let changed = app
        .state::<crate::state::AppState>()
        .data
        .reset(seed.settings, seed.registry)?;
    crate::data::commands::announce(app, changed);
    Ok(())
}

/// `LAUNCHER_WINDOW_FRAME="x,y,width,height"` (logical points) places the main window.
pub fn window_frame() -> Option<(f64, f64, f64, f64)> {
    parse_frame(&std::env::var("LAUNCHER_WINDOW_FRAME").ok()?)
}

/// `LAUNCHER_BACKGROUND=1` keeps the windows from taking focus.
pub fn background() -> bool {
    std::env::var("LAUNCHER_BACKGROUND").is_ok_and(|v| v == "1")
}

/// Runs JavaScript sent by `scripts/dev/js.sh` in the main window, or in another one named by a first
/// line `@<label>`.
pub fn eval(app: &tauri::AppHandle, js: &str) {
    use tauri::Manager;
    let (label, js) = match js.strip_prefix('@').and_then(|rest| rest.split_once('\n')) {
        Some((label, rest)) => (label.trim(), rest),
        None => (crate::window::MAIN, js),
    };
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    if let Err(e) = window.eval(js) {
        log::warn!("Couldn't evaluate script from scripts/dev: {e}");
    }
}

pub(crate) fn parse_frame(raw: &str) -> Option<(f64, f64, f64, f64)> {
    let v: Vec<f64> = raw
        .split(',')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    if !v.iter().all(|n| n.is_finite()) {
        return None;
    }
    match v[..] {
        [x, y, w, h] if w > 0.0 && h > 0.0 => Some((x, y, w, h)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frames_and_rejects_garbage() {
        assert_eq!(
            parse_frame("-3000, -537, 1400,900"),
            Some((-3000.0, -537.0, 1400.0, 900.0))
        );
        for raw in ["1,2,3", "1,2,0,4", "a,b,c,d", "1,2,3,4,5", ""] {
            assert_eq!(parse_frame(raw), None, "{raw}");
        }
    }

    #[test]
    fn non_finite_or_non_positive_sizes_are_rejected() {
        for raw in ["NaN,0,10,10", "0,0,inf,10", "0,0,-5,10", "0,0,10,0"] {
            assert_eq!(parse_frame(raw), None, "{raw}");
        }
    }

    #[test]
    fn the_first_run_writes_the_demo_seed_and_the_data_opens_with_it() {
        let config = crate::test_support::temp_config_dir();

        seed_first_run(config.path()).unwrap();

        assert!(config.path().join(demo::DEMO_FOLDER).is_dir());
        let data = crate::data::Data::open_with_home(config.path(), None);
        assert_eq!(data.projects().len(), 7);
        assert_eq!(data.settings().tags, ["Client", "Personal", "Experiments", "Ideas"]);
    }

    #[test]
    fn a_data_folder_with_a_projects_list_is_not_seeded_again() {
        let config = crate::test_support::temp_config_dir();
        std::fs::write(config.path().join(PROJECTS_FILE), r#"{"projects":[]}"#).unwrap();

        seed_first_run(config.path()).unwrap();

        assert!(!config.path().join(demo::DEMO_FOLDER).exists());
        assert!(!config.path().join(SETTINGS_FILE).exists());
        let data = crate::data::Data::open_with_home(config.path(), None);
        assert!(data.projects().is_empty());
    }

    #[test]
    fn whitespace_around_numbers_is_ignored() {
        assert_eq!(parse_frame("  1 , 2 ,  3,4 "), Some((1.0, 2.0, 3.0, 4.0)));
    }
}
