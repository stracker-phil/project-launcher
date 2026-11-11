//! Every Tauri command the page can invoke.

use tauri::ipc::Invoke;

use crate::{bus, commands, data, menu, status};

pub fn handlers() -> impl Fn(Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::open_url,
        commands::take_launch_events,
        commands::log_frontend_error,
        bus::app::run_command,
        menu::palette::menu_commands,
        status::app_status,
        data::commands::get_settings,
        data::commands::list_projects,
        data::commands::reload_projects,
        data::commands::set_appearance,
        data::commands::set_shortcut,
        data::commands::reset_shortcut,
        data::commands::add_tag,
        data::commands::rename_tag,
        data::commands::delete_tag,
        data::commands::set_tag_order,
        data::commands::add_project,
        data::commands::remove_project,
        data::commands::set_project_name,
        data::commands::set_project_meta,
        data::commands::undo_change,
        data::commands::redo_change,
    ]
}
