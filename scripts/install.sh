#!/bin/zsh
# Installs the release build as the one Launcher of this machine: ~/Applications/Launcher.app links to
# it and the `launcher` command links through that app. A running release app restarts on it, its window
# shown or hidden as before; a running debug app is left alone.
# `bun run release` ends here; debug builds are never installed (scripts/dev/launch.sh runs them by path).
# A ~/.local/bin/launcher that isn't this script's link stays unless
# `scripts/install.sh --replace-cli`.
set -e
REPLACE_CLI=
[[ "$1" == --replace-cli ]] && REPLACE_CLI=1
ROOT=$(cd "$(dirname "$0")/.." && pwd)
APP="$ROOT/src-tauri/target/release/bundle/macos/Launcher.app"
[[ -d "$APP" ]] || { echo "Not found: $APP (run 'bun run release')"; exit 1; }
LINK="$HOME/Applications/Launcher.app"
BIN="$LINK/Contents/MacOS/launcher"
CLI="$HOME/.local/bin/launcher"

source "$ROOT/scripts/app-pids.sh"
running() { app_pids "$APP/Contents/MacOS/launcher" >/dev/null; }
# Re-signing a running app's files can get it killed, so it quits first; a hidden window stays hidden.
if running; then
  WAS_RUNNING=1
  OPEN_ARGS=()
  "$APP/Contents/MacOS/launcher" status --json 2>/dev/null | grep -q '"windowVisible": *false' \
    && OPEN_ARGS=(--args --hidden)
  "$ROOT/scripts/stop-app.sh" "$APP/Contents/MacOS/launcher"
fi
"$ROOT/scripts/sign-app.sh" "$APP" com.philippstracker.launcher

mkdir -p "$HOME/Applications" "$HOME/.local/bin"
ln -sfn "$APP" "$LINK"
echo "Installed the release build as ~/Applications/Launcher.app"
if [[ ! -e "$CLI" && ! -L "$CLI" ]] || [[ "$(readlink "$CLI")" == "$BIN" ]] || [[ -n "$REPLACE_CLI" ]]; then
  ln -sfn "$BIN" "$CLI"
  echo "Linked ~/.local/bin/launcher"
else
  echo "Left ~/.local/bin/launcher alone (not this script's link); replace it with: scripts/install.sh --replace-cli"
fi

[[ -n "$WAS_RUNNING" ]] || exit 0
# Launch Services can refuse right after the old app quit (error -600), so try a few times.
for _ in {1..10}; do
  open -g "$LINK" "${OPEN_ARGS[@]}" 2>/dev/null || true
  sleep 1
  if running; then
    # Opened by Launch Services it starts in the menu bar (ADR 0044); a window that was open comes back.
    [[ ${#OPEN_ARGS[@]} -eq 0 ]] && { sleep 2; "$BIN" show >/dev/null 2>&1 || true; }
    echo "Restarted Launcher on the new release build"
    exit 0
  fi
done
echo "Launcher did not start again; open it from ~/Applications" >&2
exit 1
