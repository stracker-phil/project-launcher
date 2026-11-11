#!/bin/zsh
# Checks that nothing jumps in the running debug app: shows the window, then opens and closes the inspector and
# hides and shows the sidebar, measuring every pane after each step. Panes that stay must keep their place while
# another opens, and everything must be back after the second toggle. Exits 1 on any drift of 1px or more.
# Start the app first: bun run build:app && scripts/dev/launch.sh
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/src-tauri/target/debug/bundle/macos/Launcher (Debug).app/Contents/MacOS/launcher"
LOG="${TMPDIR:-/tmp}/launcher-dev.log"
MARK="E2E-$(date +%s)-$$"
"$BIN" status >/dev/null 2>&1 || { echo "The debug app isn't running: scripts/dev/launch.sh"; exit 1; }
sleep 1

"$ROOT/scripts/dev/js.sh" "(async () => {
  const report = (text) => window.__TAURI_INTERNALS__.invoke('log_frontend_error', { message: '$MARK ' + text });
  const wait = (ms) => new Promise((done) => setTimeout(done, ms));
  const store = window.launcher;
  const rect = (selector) => {
    const el = document.querySelector(selector);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return [r.x, r.y, r.width, r.height].map(Math.round).join(',');
  };
  const parts = () => ({
    sidebar: rect('.sidebar'),
    list: rect('.list-pane'),
    detailStrip: rect('.detail-pane .pane__strip'),
    inspector: rect('.inspector-pane'),
  });
  const drift = [];
  const compare = (step, before, after, names) => {
    for (const name of names) if (before[name] !== after[name]) drift.push(step + ': ' + name + ' ' + before[name] + ' -> ' + after[name]);
  };
  const toggleTwice = async (name, toggle, stay) => {
    const before = parts();
    toggle();
    await wait(400);
    compare(name + ' on', before, parts(), stay);
    toggle();
    await wait(400);
    compare(name + ' off', before, parts(), Object.keys(before));
  };
  try {
    if (!store) throw new Error('window.launcher is missing (build with bun run build:app)');
    await toggleTwice('inspector', () => store.getState().toggleInspector(), ['sidebar', 'list']);
    await toggleTwice('sidebar', () => store.getState().toggleSidebar(), []);
    report(drift.length === 0 ? 'OK' : 'DRIFT ' + drift.join(' | '));
  } catch (error) {
    report('ERROR ' + error);
  }
})()"

for _ in {1..30}; do
  sleep 0.5
  line=$(grep -a -o "$MARK [^\"]*" "$LOG" | head -1) && [[ -n "$line" ]] && break
done
result=${line#"$MARK "}
case "$result" in
  OK) echo "No drift"; exit 0 ;;
  "") echo "No result in $LOG"; exit 1 ;;
  *) echo "${result//' | '/$'\n'}"; exit 1 ;;
esac
