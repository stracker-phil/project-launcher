//! The menu-bar item, the app's home while every window is hidden (ADR 0034).

use tauri::AppHandle;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

use crate::window;

const TRAY_ID: &str = "main";
const OPEN: &str = "tray:open";
const QUIT: &str = "tray:quit";

/// 36 px source for the 18 pt menu bar icon.
const ICON: &[u8] = include_bytes!("../icons/tray/tray.png");

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(
                app,
                OPEN,
                format!("Open {}", crate::APP_NAME),
                true,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                QUIT,
                format!("Quit {}", crate::APP_NAME),
                true,
                None::<&str>,
            )?,
        ],
    )?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON)?)
        .icon_as_template(true)
        .tooltip(crate::APP_NAME)
        .menu(&menu)
        .on_menu_event(|app, event| clicked(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

fn clicked(app: &AppHandle, id: &str) {
    match id {
        OPEN => window::show_main(app),
        QUIT => app.exit(0),
        _ => {}
    }
}
