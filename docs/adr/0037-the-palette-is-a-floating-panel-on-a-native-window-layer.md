# 0037. The palette is a floating panel on a native window layer

## Status

Accepted (2026-11-11). Inherited from the family catalog (app-blueprint: `native-window-layer`, Optional); reference implementation: Daybook ADR 0016 and ADR 0027.

## Context

The prototype's palette was a floating panel: it opened from the tray and the global shortcut while the main
window was hidden, ran a command and handed focus back to the app the owner came from. Tauri windows cannot be
non-activating panels on their own, and the prototype hit focus-bounce and visibility quirks.

## Decision

The palette is its own window (`palette`), turned into a non-activating `NSPanel` through one objc2 layer
(`src-tauri/src/window/native.rs`): window level above normal windows, all Spaces, no Dock icon. It is
centred over the main window when that is visible, else on the active screen; placement rules are pure and
unit-tested, applied on the main thread, shown after a ready/painted handshake at alpha 0. It closes on Esc,
⌘K and blur, and focus returns only to an app on the current Space. Its page is `src/windows/palette/` with
its own Vite input; it never navigates the main window.

- Rejected: the family's in-window palette (it needs the main window shown first); Electron-style panel
  options alone (no such API in Tauri).

## Consequences

- The palette works with every window hidden, as in the prototype.
- objc2 code and a pinned `tao` may be needed, as in Daybook; panel behavior is verified in the running app.
