//! Building the menu bar from the items: only known commands, enabled as the bus says, with macOS's
//! standard items around them.

use tauri::menu::{AboutMetadata, Menu, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

use super::items::{
    APP_MENU, EDIT_TITLE, FILE_MENU, HELP_MENU, Item, MenuDef, VIEW_MENU, WINDOW_TITLE,
};
use super::shown_groups;
use crate::bus::{self, CommandStatus};

struct Ctx<'a> {
    app: &'a AppHandle,
    commands: &'a dyn CommandStatus,
}

type Builder<'a> = SubmenuBuilder<'a, Wry, AppHandle>;

fn with_items<'a>(
    ctx: &Ctx<'a>,
    mut builder: Builder<'a>,
    items: &[&Item],
) -> tauri::Result<Builder<'a>> {
    for item in items {
        let mut menu_item = MenuItemBuilder::with_id(item.command, item.label)
            .enabled(ctx.commands.is_enabled(item.command));
        if let Some(accelerator) = item.accelerator {
            menu_item = menu_item.accelerator(accelerator);
        }
        builder = builder.item(&menu_item.build(ctx.app)?);
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
    let ctx = Ctx {
        app,
        commands: &commands,
    };
    let launcher = app_submenu(&ctx)?;
    let file = submenu(&ctx, FILE_MENU)?.build()?;
    let edit = SubmenuBuilder::new(app, EDIT_TITLE)
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = submenu(&ctx, VIEW_MENU)?.separator().fullscreen().build()?;
    let window = SubmenuBuilder::new(app, WINDOW_TITLE)
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    let help = submenu(&ctx, HELP_MENU)?.build()?;
    Menu::with_items(app, &[&launcher, &file, &edit, &view, &window, &help])
}
