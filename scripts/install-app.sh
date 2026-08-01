#!/bin/sh
set -eu

# Installs the release bundle produced by `tauri build` into /Applications,
# replacing any existing copy. Run the build first (see the app:install script).

app="Just Notes.app"
src="src-tauri/target/release/bundle/macos/$app"
dst="/Applications/$app"
process="$dst/Contents/MacOS/just-notes"
process_pattern='^/Applications/Just Notes[.]app/Contents/MacOS/just-notes([[:space:]]|$)'

installed_pids() {
  for pid in $(pgrep -f "$process_pattern" 2>/dev/null || true); do
    if lsof -a -p "$pid" -d txt -Fn 2>/dev/null | grep -Fqx "n$process"; then
      echo "$pid"
    fi
  done
}

is_running() {
  [ -n "$(installed_pids)" ]
}

signal_running() {
  for pid in $(installed_pids); do
    kill "-$1" "$pid" >/dev/null 2>&1 || true
  done
}

wait_for_exit() {
  attempts=50
  while is_running && [ "$attempts" -gt 0 ]; do
    sleep 0.1
    attempts=$((attempts - 1))
  done
  ! is_running
}

if [ ! -d "$src" ]; then
  echo "Build output not found at $src — run \`tauri build\` first." >&2
  exit 1
fi

# Quit a running copy so its bundle isn't replaced underneath it.
osascript -e 'quit app "Just Notes"' >/dev/null 2>&1 || true
if ! wait_for_exit; then
  signal_running TERM
fi
if ! wait_for_exit; then
  signal_running KILL
fi
if ! wait_for_exit; then
  echo "Could not stop the installed Just Notes process." >&2
  exit 1
fi

rm -rf "$dst"
ditto "$src" "$dst"
open "$dst"
echo "Installed and launched $app from $src."
