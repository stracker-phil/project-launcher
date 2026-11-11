//! The palette window as a non-activating `NSPanel`, like Spotlight: it takes the keyboard without activating
//! Launcher, so the app in front stays in front (ADR 0037). `PanelWindow` is only made on the main thread.

use std::ffi::{CStr, c_void};
use std::sync::OnceLock;

use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};

const NONACTIVATING_PANEL: usize = 1 << 7;
const MOVE_TO_ACTIVE_SPACE: usize = 1 << 1;
const IGNORES_CYCLE: usize = 1 << 6;
const FULL_SCREEN_AUXILIARY: usize = 1 << 8;
/// `NSPopUpMenuWindowLevel`: above other apps' windows, also full-screen ones.
const POP_UP_MENU_LEVEL: isize = 101;
/// The ivar tao's `TaoWindow` adds; the panel class repeats it so the swapped window keeps its layout.
const FOCUSABLE: &CStr = c"focusable";

extern "C-unwind" fn yes(_: &AnyObject, _: Sel) -> Bool {
    Bool::YES
}

extern "C-unwind" fn no(_: &AnyObject, _: Sel) -> Bool {
    Bool::NO
}

/// An `NSPanel` that takes the keyboard although it has no title bar, and never becomes the main window.
fn panel_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASS.get_or_init(|| {
        let mut builder = ClassBuilder::new(c"LauncherPanel", class!(NSPanel))
            .expect("the panel class is declared once");
        builder.add_ivar::<Bool>(FOCUSABLE);
        // SAFETY: both selectors take no arguments and return BOOL.
        unsafe {
            builder.add_method(
                sel!(canBecomeKeyWindow),
                yes as extern "C-unwind" fn(_, _) -> _,
            );
            builder.add_method(
                sel!(canBecomeMainWindow),
                no as extern "C-unwind" fn(_, _) -> _,
            );
        }
        builder.register()
    })
}

/// Where `class` or one of its superclasses keeps the `FOCUSABLE` ivar.
fn focusable_offset(class: &AnyClass) -> Option<isize> {
    std::iter::successors(Some(class), |class| class.superclass())
        .find_map(|class| class.instance_variable(FOCUSABLE))
        .map(|ivar| ivar.offset())
}

/// Whether an object of `from` can become the panel in place: same size and the same ivar where it was.
fn fits_panel(from: &AnyClass) -> bool {
    let panel = panel_class();
    let Some(focusable) = focusable_offset(from) else {
        return false;
    };
    from.instance_size() == panel.instance_size() && focusable_offset(panel) == Some(focusable)
}

/// A live `NSWindow`, reached on the main thread.
pub struct PanelWindow<'a>(&'a AnyObject);

impl PanelWindow<'_> {
    /// # Safety
    /// `ns_window` is a live `NSWindow`, used only on the main thread while it lives.
    pub unsafe fn new(ns_window: *mut c_void) -> Self {
        // SAFETY: as the caller promises.
        PanelWindow(unsafe { &*ns_window.cast::<AnyObject>() })
    }

    /// Turns the window into the panel: non-activating, above other apps on the active Space, invisible until
    /// shown. A window whose class has an unexpected layout stays a plain window.
    pub fn make_panel(&self) {
        let window = self.0;
        let panel = panel_class();
        if window.class() != panel {
            if !fits_panel(window.class()) {
                log::warn!(
                    "The palette stays a plain window: {:?} has an unexpected layout",
                    window.class().name()
                );
                return;
            }
            // SAFETY: checked above that the panel class matches the window's size and ivar.
            unsafe { AnyObject::set_class(window, panel) };
        }
        // SAFETY: plain NSWindow / NSPanel setters on a live window.
        unsafe {
            let mask: usize = msg_send![window, styleMask];
            let _: () = msg_send![window, setStyleMask: mask | NONACTIVATING_PANEL];
            let _: () = msg_send![window, setHidesOnDeactivate: false];
            let behavior = MOVE_TO_ACTIVE_SPACE | FULL_SCREEN_AUXILIARY | IGNORES_CYCLE;
            let _: () = msg_send![window, setCollectionBehavior: behavior];
            let _: () = msg_send![window, setLevel: POP_UP_MENU_LEVEL];
        }
        self.set_alpha(0.0);
    }

    pub fn set_alpha(&self, alpha: f64) {
        // SAFETY: plain NSWindow setter on a live window.
        unsafe {
            let _: () = msg_send![self.0, setAlphaValue: alpha];
        }
    }

    /// Brings the window to the front and, with `keyboard`, gives it the keys without activating the app.
    pub fn show(&self, keyboard: bool) {
        // SAFETY: plain NSWindow calls on a live window.
        unsafe {
            let _: () = msg_send![self.0, orderFrontRegardless];
            if keyboard {
                let _: () = msg_send![self.0, makeKeyWindow];
            }
        }
    }

    /// Invisible first, so whatever macOS does with the window afterwards, a closed palette isn't seen.
    pub fn hide(&self) {
        self.set_alpha(0.0);
        let none: Option<&AnyObject> = None;
        // SAFETY: plain NSWindow call on a live window.
        unsafe {
            let _: () = msg_send![self.0, orderOut: none];
        }
    }
}
