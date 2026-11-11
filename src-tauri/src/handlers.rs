//! Every Tauri command the page can invoke.

use tauri::ipc::Invoke;

use crate::{bus, commands, menu, status};

pub fn handlers() -> impl Fn(Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::open_url,
        commands::take_launch_events,
        commands::log_frontend_error,
        bus::app::run_command,
        menu::palette::menu_commands,
        status::app_status,
    ]
}
