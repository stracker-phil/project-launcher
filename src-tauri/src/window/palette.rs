//! The command palette's window: a floating panel over the main window, or on the pointer's screen while that
//! is hidden (ADR 0037). It opens invisible, the page resets and reports back (`palette_shown`), then it shows;
//! Esc, a chosen row or a click elsewhere closes it.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use super::native::PanelWindow;

use crate::events::{PALETTE_CLOSED_EVENT, PALETTE_OPENED_EVENT};
use crate::lock::MutexExt;
use crate::state::AppState;

pub const LABEL: &str = "palette";
const WIDTH: f64 = 680.0;
const HEIGHT: f64 = 460.0;
/// A second toggle this soon after the first is the same key press arriving twice (menu and page).
const DEBOUNCE: Duration = Duration::from_millis(250);
/// Losing the focus this soon after opening is macOS settling the panel, not a click elsewhere.
const SETTLE: Duration = Duration::from_millis(400);

/// A rectangle in logical points, origin at the top left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The palette's top-left corner: centred over the main window while it shows, else on `screen`, a fifth of
/// the way down; always inside `screen`.
pub fn place(main: Option<Rect>, screen: Rect, width: f64, height: f64) -> (f64, f64) {
    let over = main.unwrap_or(screen);
    let x = over.x + (over.width - width) / 2.0;
    let y = over.y + over.height / 5.0;
    let clamp =
        |value: f64, start: f64, room: f64, size: f64| value.min(start + room - size).max(start);
    (
        clamp(x, screen.x, screen.width, width).round(),
        clamp(y, screen.y, screen.height, height).round(),
    )
}

/// Whether the palette is open, and when it opened and was last toggled.
#[derive(Default)]
struct State {
    open: bool,
    opened_at: Option<Instant>,
    toggled_at: Option<Instant>,
}

#[derive(Default)]
pub struct Palette {
    state: Mutex<State>,
}

impl Palette {
    fn is_open(&self) -> bool {
        self.state.lock_unpoisoned().open
    }

    /// Whether it opened less than `SETTLE` ago.
    fn settling(&self) -> bool {
        self.state
            .lock_unpoisoned()
            .opened_at
            .is_some_and(|at| at.elapsed() < SETTLE)
    }

    fn mark_opened(&self) {
        let mut state = self.state.lock_unpoisoned();
        state.open = true;
        state.opened_at = Some(Instant::now());
    }

    /// Returns whether it was open.
    fn mark_closed(&self) -> bool {
        std::mem::replace(&mut self.state.lock_unpoisoned().open, false)
    }

    /// Whether it is open; `None` for a second toggle within `DEBOUNCE`.
    fn toggled(&self) -> Option<bool> {
        let mut state = self.state.lock_unpoisoned();
        if state.toggled_at.is_some_and(|at| at.elapsed() < DEBOUNCE) {
            return None;
        }
        state.toggled_at = Some(Instant::now());
        Some(state.open)
    }
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Builds the hidden palette window and turns it into the panel; once, at startup.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("src/windows/palette/index.html".into()),
    )
    .title("Command Palette")
    .inner_size(WIDTH, HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;
    on_panel(&window, |panel| panel.make_panel());
    Ok(())
}

/// Runs `work` on the main thread with the window's `NSWindow`.
fn on_panel(window: &WebviewWindow, work: impl FnOnce(&PanelWindow) + Send + 'static) {
    let target = window.clone();
    let result = window.run_on_main_thread(move || match target.ns_window() {
        // SAFETY: on the main thread, with the window alive for the call.
        Ok(ns_window) => work(&unsafe { PanelWindow::new(ns_window) }),
        Err(e) => log::warn!("The palette has no native window: {e}"),
    });
    if let Err(e) = result {
        log::warn!("Couldn't reach the main thread for the palette: {e}");
    }
}

impl Rect {
    fn new(position: LogicalPosition<f64>, size: LogicalSize<f64>) -> Self {
        Rect {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }
    }
}

fn work_area(monitor: &tauri::Monitor) -> Rect {
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    Rect::new(area.position.to_logical(scale), area.size.to_logical(scale))
}

/// The main window's frame and screen while it shows.
fn main_frame(app: &AppHandle) -> Option<(Rect, Rect)> {
    let main = app.get_webview_window(super::MAIN)?;
    if !main.is_visible().ok()? || main.is_minimized().ok()? {
        return None;
    }
    let scale = main.scale_factor().ok()?;
    let frame = Rect::new(
        main.outer_position().ok()?.to_logical(scale),
        main.outer_size().ok()?.to_logical(scale),
    );
    Some((frame, work_area(&main.current_monitor().ok()??)))
}

/// The screen under the pointer, else the primary one.
fn pointer_screen(app: &AppHandle) -> Option<Rect> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    Some(work_area(&monitor))
}

/// Opens the palette, or closes it while it is open.
pub fn toggle(app: &AppHandle) {
    match app.state::<Palette>().toggled() {
        Some(true) => close(app),
        Some(false) => open_palette(app),
        None => {}
    }
}

fn open_palette(app: &AppHandle) {
    let Some(window) = window(app) else { return };
    let main = main_frame(app);
    let screen = main
        .map(|(_, screen)| screen)
        .or_else(|| pointer_screen(app));
    if let Some(screen) = screen {
        let (x, y) = place(main.map(|(frame, _)| frame), screen, WIDTH, HEIGHT);
        if let Err(e) = window.set_position(LogicalPosition::new(x, y)) {
            log::warn!("Couldn't place the palette: {e}");
        }
    }
    app.state::<Palette>().mark_opened();
    let keyboard = takes_keyboard();
    on_panel(&window, move |panel| {
        panel.set_alpha(0.0);
        panel.show(keyboard);
    });
    let current = app.state::<AppState>().current_project();
    if let Err(e) = app.emit_to(LABEL, PALETTE_OPENED_EVENT, current) {
        log::warn!("Couldn't tell the palette it opened: {e}");
    }
}

/// The page has drawn its fresh state: the palette becomes visible.
pub fn shown(app: &AppHandle) {
    if !app.state::<Palette>().is_open() {
        return;
    }
    if let Some(window) = window(app) {
        on_panel(&window, |panel| panel.set_alpha(1.0));
    }
}

pub fn close(app: &AppHandle) {
    if !app.state::<Palette>().mark_closed() {
        return;
    }
    if let Some(window) = window(app) {
        on_panel(&window, |panel| panel.hide());
    }
    if let Err(e) = app.emit_to(LABEL, PALETTE_CLOSED_EVENT, ()) {
        log::warn!("Couldn't tell the palette it closed: {e}");
    }
}

/// A check that captures the palette (`LAUNCHER_PALETTE_PASSIVE=1`, debug builds only) keeps it from taking the
/// keys of the app in front and from closing when the capture tool takes the focus.
fn passive() -> bool {
    cfg!(debug_assertions) && std::env::var("LAUNCHER_PALETTE_PASSIVE").as_deref() == Ok("1")
}

fn takes_keyboard() -> bool {
    !passive()
}

/// A click elsewhere closes the palette, unless it is still settling after opening.
pub fn focus_lost(app: &AppHandle) {
    if passive() {
        return;
    }
    if app.state::<Palette>().settling() {
        if let Some(window) = window(app) {
            let keyboard = takes_keyboard();
            on_panel(&window, move |panel| panel.show(keyboard));
        }
    } else {
        close(app);
    }
}

#[tauri::command]
pub fn palette_shown(app: AppHandle) {
    shown(&app);
}

#[tauri::command]
pub fn close_palette(app: AppHandle) {
    close(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn screen() -> Rect {
        rect(0.0, 0.0, 1440.0, 900.0)
    }

    #[test]
    fn the_palette_is_centred_over_the_main_window_a_fifth_of_the_way_down() {
        let main = rect(100.0, 50.0, 1000.0, 700.0);
        assert_eq!(place(Some(main), screen(), WIDTH, HEIGHT), (260.0, 190.0));
    }

    #[test]
    fn without_a_main_window_the_palette_is_centred_on_the_screen_a_fifth_down() {
        assert_eq!(place(None, screen(), WIDTH, HEIGHT), (380.0, 180.0));
    }

    #[test]
    fn a_screen_away_from_the_origin_places_the_palette_relative_to_its_own_corner() {
        let second = rect(1440.0, 25.0, 1440.0, 875.0);
        assert_eq!(place(None, second, WIDTH, HEIGHT), (1820.0, 200.0));
    }

    #[test]
    fn a_main_window_hanging_off_an_edge_keeps_the_palette_inside_the_screen() {
        for (name, main, expected) in [
            ("left", rect(-600.0, 50.0, 1000.0, 700.0), (0.0, 190.0)),
            ("right", rect(1200.0, 50.0, 1000.0, 700.0), (760.0, 190.0)),
            ("bottom", rect(100.0, 700.0, 1000.0, 700.0), (260.0, 440.0)),
            ("top", rect(100.0, -300.0, 1000.0, 700.0), (260.0, 0.0)),
        ] {
            assert_eq!(
                place(Some(main), screen(), WIDTH, HEIGHT),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn a_palette_larger_than_the_screen_sticks_to_its_top_left_corner() {
        let small = rect(10.0, 20.0, 600.0, 400.0);
        assert_eq!(place(None, small, WIDTH, HEIGHT), (10.0, 20.0));
    }

    #[test]
    fn the_position_is_rounded_to_whole_points() {
        let main = rect(0.0, 0.0, 1001.0, 903.0);
        assert_eq!(place(Some(main), screen(), WIDTH, HEIGHT), (161.0, 181.0));
    }
}
