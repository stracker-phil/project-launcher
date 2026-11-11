#!/bin/zsh
# Captures the README screenshots, light and dark, from the debug app on a fresh demo seed: screens.sh [dir]
# (default docs/screens). Moves the debug data folder aside, starts the debug build at a fixed size, drives it
# with its own CLI and restores the folder afterwards. The window comes to the front: macOS doesn't draw a
# covered web view, so a capture of it is blank. Build first: bun run build:app.
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${1:-$ROOT/docs/screens}
HERE="$ROOT/scripts/dev"
APP="$ROOT/src-tauri/target/debug/bundle/macos/Launcher (Debug).app"
BIN="$APP/Contents/MacOS/launcher"
DATA="$HOME/Library/Application Support/com.philippstracker.launcher.debug"
ASIDE="$DATA.screens-backup"
FRAME=(80 60 1440 900)
[[ -x "$BIN" ]] || { echo "Build first: bun run build:app"; exit 1; }
[[ ! -e "$ASIDE" ]] || { echo "$ASIDE exists from an earlier run; restore or remove it first"; exit 1; }
mkdir -p "$OUT"

"$ROOT/scripts/stop-app.sh" "$BIN"
[[ -e "$DATA" ]] && mv "$DATA" "$ASIDE"
restore() {
  "$ROOT/scripts/stop-app.sh" "$BIN"
  rm -rf "$DATA"
  [[ -e "$ASIDE" ]] && mv "$ASIDE" "$DATA"
}
trap restore EXIT

"$ROOT/scripts/sign-app.sh" "$APP" com.philippstracker.launcher.debug
LAUNCHER_WINDOW_FRAME="${(j:,:)FRAME}" nohup "$BIN" >/dev/null 2>&1 &
disown
for _ in {1..30}; do
  sleep 1
  "$BIN" status >/dev/null 2>&1 && break
done
"$BIN" focus >/dev/null
sleep 3

js() { "$HERE/js.sh" "$1"; sleep 0.6; }
invoke() { js "window.__TAURI_INTERNALS__.invoke($1)"; }
shot() { sleep 1.5; "$HERE/screenshot.sh" "$OUT/$1.png"; echo "$OUT/$1.png"; }
# The main window's area of the screen, with whatever floats over it (the palette).
shot_area() { sleep 1.5; screencapture -x -R"${(j:,:)FRAME}" "$OUT/$1.png"; echo "$OUT/$1.png"; }
appearance() { invoke "'set_appearance', { appearance: '$1' }"; sleep 0.5; }
select_project() { "$BIN" open "$1" >/dev/null; sleep 1; }
inspector() { js "(() => { const s = window.launcher.getState(); if (!s.inspectorOpen.withSidebar) s.toggleInspector(); })()"; }

# Acme Website with its DDEV server running, the inspector open.
appearance dark
select_project "Acme Website"
js '[...document.querySelectorAll(".dev-row button")].find((b) => b.textContent === "Start")?.click()'
sleep 3
inspector
shot main-dark

appearance light
shot main-light

# The action editor on Edit Code, from the tile's menu.
js '(() => { const tile = document.querySelector(".action-tile"); const r = tile.getBoundingClientRect(); tile.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: r.left + 40, clientY: r.bottom - 6 })); })()'
js '[...document.querySelectorAll("[role=menuitem]")].find((e) => e.textContent.startsWith("Edit"))?.click()'
shot editor-light
js 'document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }))'

# The command palette over the main window, searching.
appearance dark
select_project "Orbit API"
"$BIN" palette >/dev/null
sleep 1
"$HERE/js.sh" $'@palette\n(() => { const input = document.querySelector(".palette input"); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set; set.call(input, "o"); input.dispatchEvent(new Event("input", { bubbles: true })); })()'
shot_area palette-dark
"$BIN" palette >/dev/null

# Settings → General, alone: the main window hides first.
"$BIN" hide >/dev/null
"$BIN" settings >/dev/null
sleep 1
shot settings-dark
