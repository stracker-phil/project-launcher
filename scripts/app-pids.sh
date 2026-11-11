# Sourced by scripts that start or stop Launcher. `app_pids path/to/launcher` lists the app process ids of
# the release or the debug build, whichever the path names, wherever it runs from: its binary without a
# subcommand (the CLI has one).
app_pids() {
  local bundle='Launcher'
  [[ "$1" == *"Launcher (Debug).app/"* ]] && bundle='Launcher \(Debug\)'
  pgrep -f "${bundle}\.app/Contents/MacOS/launcher( --hidden)?$"
}
