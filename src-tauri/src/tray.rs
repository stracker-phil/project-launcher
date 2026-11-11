//! The menu-bar item, the app's home while every window is hidden (ADR 0034). Its items' ids are bus
//! commands; the app's menu event handler (`menu::clicked`) runs them, as it does the menu bar's.

use tauri::AppHandle;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

const TRAY_ID: &str = "main";
const OPEN: &str = "app:show";
const QUIT: &str = "app:quit";

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
    // No `on_menu_event` here: Tauri hands tray menu clicks to the app's handler too, so they would run twice.
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON)?)
        .icon_as_template(true)
        .tooltip(crate::APP_NAME)
        .menu(&menu)
        .build(app)?;
    Ok(())
}
