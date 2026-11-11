//! Building the menu bar from the items: only known commands, enabled as the bus says, keys with the owner's
//! overrides, and macOS's standard items around them.

use tauri::menu::{AboutMetadata, Menu, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

use super::items::{
    APP_MENU, EDIT_MENU, FILE_MENU, HELP_MENU, Item, LIBRARY_MENU, MenuDef, PROJECT_MENU,
    VIEW_MENU, WINDOW_TITLE,
};
use super::keys::effective_accelerator;
use super::{MenuKeys, shown_groups};
use crate::bus::{self, CommandStatus};
use crate::state::AppState;
use crate::store::Shortcuts;

struct Ctx<'a> {
    app: &'a AppHandle,
    commands: &'a dyn CommandStatus,
    /// `None` while Settings → Shortcuts records a key, so the menu doesn't take it.
    shortcuts: Option<&'a Shortcuts>,
}

impl Ctx<'_> {
    fn accelerator(&self, item: &Item) -> Option<String> {
        self.shortcuts
            .and_then(|shortcuts| effective_accelerator(item, shortcuts))
    }
}

type Builder<'a> = SubmenuBuilder<'a, Wry, AppHandle>;

fn with_items<'a>(
    ctx: &Ctx<'a>,
    mut builder: Builder<'a>,
    items: &[&Item],
) -> tauri::Result<Builder<'a>> {
    for item in items {
        let menu_item = || {
            MenuItemBuilder::with_id(item.command, item.label)
                .enabled(ctx.commands.is_enabled(item.command))
        };
        let built = match ctx.accelerator(item) {
            Some(accelerator) => menu_item()
                .accelerator(&accelerator)
                .build(ctx.app)
                .or_else(|e| {
                    log::warn!("Menu item {} can't take {accelerator}: {e}", item.command);
                    menu_item().build(ctx.app)
                })?,
            None => menu_item().build(ctx.app)?,
        };
        builder = builder.item(&built);
    }
    Ok(builder)
}

/// The menu's shown groups with a separator between each two.
fn submenu<'a>(ctx: &Ctx<'a>, (title, groups): MenuDef) -> tauri::Result<Builder<'a>> {
    let mut builder = SubmenuBuilder::new(ctx.app, title);
    for (i, items) in shown_groups(groups, ctx.commands).iter().enumerate() {
        if i > 0 {
            builder = builder.separator();
        }
        builder = with_items(ctx, builder, items)?;
    }
    Ok(builder)
}

fn app_submenu(ctx: &Ctx) -> tauri::Result<Submenu<Wry>> {
    let mut builder = SubmenuBuilder::new(ctx.app, APP_MENU.0)
        .about(Some(AboutMetadata::default()))
        .separator();
    // Settings above Services, Quit below Hide.
    let (above, below) = APP_MENU.1.split_at(1);
    for items in shown_groups(above, ctx.commands) {
        builder = with_items(ctx, builder, &items)?.separator();
    }
    builder = builder
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator();
    for items in shown_groups(below, ctx.commands) {
        builder = with_items(ctx, builder, &items)?;
    }
    builder.build()
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let registry = app.state::<bus::Commands>();
    let commands = registry.available(app);
    let shortcuts = app.state::<AppState>().data.settings().shortcuts;
    let ctx = Ctx {
        app,
        commands: &commands,
        shortcuts: (!app.state::<MenuKeys>().paused()).then_some(&shortcuts),
    };
    let launcher = app_submenu(&ctx)?;
    let file = submenu(&ctx, FILE_MENU)?.build()?;
    let edit = submenu(&ctx, EDIT_MENU)?
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = submenu(&ctx, VIEW_MENU)?.separator().fullscreen().build()?;
    let library = submenu(&ctx, LIBRARY_MENU)?.build()?;
    let project = submenu(&ctx, PROJECT_MENU)?.build()?;
    let window = SubmenuBuilder::new(app, WINDOW_TITLE)
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = submenu(&ctx, HELP_MENU)?.build()?;
    Menu::with_items(
        app,
        &[
            &launcher, &file, &edit, &view, &library, &project, &window, &help,
        ],
    )
}
