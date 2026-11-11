#!/bin/zsh
# Captures the running debug app's window in each palette (docs/ux.md → Palette checks), whatever macOS is set
# to: screenshot-looks.sh <out-dir> writes <out-dir>/light.png and <out-dir>/dark.png, then reloads the page so
# the window follows the Appearance setting again.
set -e
DIR=${1:?usage: screenshot-looks.sh <out-dir>}
HERE=${0:A:h}
mkdir -p "$DIR"
for scheme in light dark; do
  "$HERE/js.sh" "document.documentElement.dataset.scheme = '$scheme'"
  sleep 0.5
  "$HERE/screenshot.sh" "$DIR/$scheme.png"
done
"$HERE/js.sh" "location.reload()"
