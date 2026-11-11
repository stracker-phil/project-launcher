//! The Tauri app: managed state, plugins, window and menu events, setup and the run loop.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

use crate::cli::LogLevel;
use crate::handlers::handlers;
use crate::ipc::IpcMessage;
use crate::state::AppState;
use crate::{bus, ipc, menu, remote, tray, window};

const MAX_LOG_BYTES: u128 = 5 * 1024 * 1024;

pub fn start(
    state: AppState,
    socket: PathBuf,
    initial: IpcMessage,
    log_level: LogLevel,
    hidden: bool,
) {
    tauri::Builder::default()
        .manage(state)
        .manage(commands_bus())
        .plugin(log_plugin())
        .plugin(tauri_plugin_opener::init())
        .plugin(window::navigation::guard())
        .plugin(tauri_plugin_deep_link::init())
        .on_menu_event(|app, event| menu::clicked(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window::close_requested(window.app_handle(), window.label())
            {
                api.prevent_close();
            }
        })
        .setup(move |app| {
            // After `log_plugin`'s setup, which raises the max level to `Trace`.
            log::set_max_level(log_level.into());
            log::info!("Launcher {} started", env!("CARGO_PKG_VERSION"));
            // A menu-bar app: no Dock icon until a window shows.
            window::set_dock_icon(app.handle(), false);
            app.set_menu(menu::build(app.handle())?)?;
            tray::install(app.handle())?;
            app.state::<AppState>().launches.push(initial);
            #[cfg(debug_assertions)]
            apply_dev_frame(app);
            start_ipc(app.handle().clone(), &socket);
            listen_for_links(app.handle());
            // The window starts invisible (tauri.conf.json), so a hidden start never flashes it.
            if !hidden {
                window::show_main(app.handle());
            }
            Ok(())
        })
        .invoke_handler(handlers())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // Hiding the last window must not end the app; only Quit does (with an exit code).
            RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            RunEvent::Reopen { .. } => window::show_main(app),
            _ => {}
        });
}

fn commands_bus() -> bus::Commands {
    let mut commands = bus::CommandBus::default();
    bus::register(&mut commands);
    bus::Commands(commands)
}

/// Links (`launcher://…`) bring the window forward; the commands they may run arrive with the command bus.
fn listen_for_links(app: &AppHandle) {
    let handle = app.clone();
    app.deep_link().on_open_url(move |_| {
        log::info!("Opened by a link");
        window::show_main(&handle);
    });
}

fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .rotation_strategy(RotationStrategy::KeepSome(3))
        .max_file_size(MAX_LOG_BYTES)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        // The dispatch caps every later `log::set_max_level`, so it lets everything through;
        // the app's setup sets the effective level.
        .level(log::LevelFilter::Trace)
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .level_for("tauri_runtime_wry", log::LevelFilter::Info)
        .build()
}

#[cfg(debug_assertions)]
fn apply_dev_frame(app: &tauri::App) {
    let (Some((x, y, w, h)), Some(window)) = (
        crate::dev::window_frame(),
        app.get_webview_window(window::MAIN),
    ) else {
        return;
    };
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
}

/// Without the socket the app still runs, only without single-instance handoff.
fn start_ipc(handle: AppHandle, socket: &Path) {
    let listener = match ipc::bind(socket) {
        Ok(listener) => listener,
        Err(e) => {
            log::warn!("Cannot listen on {}: {e}", socket.display());
            return;
        }
    };
    tauri::async_runtime::spawn(ipc::serve(listener, move |message| {
        remote::handle(handle.clone(), message)
    }));
}
