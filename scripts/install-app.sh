#!/bin/sh
set -eu

# Installs the release bundle produced by `tauri build` into /Applications,
# replacing any existing copy. Run the build first (see the app:install script).

app="Just Notes.app"
src="src-tauri/target/release/bundle/macos/$app"
dst="/Applications/$app"

if [ ! -d "$src" ]; then
  echo "Build output not found at $src — run \`tauri build\` first." >&2
  exit 1
fi

# Quit a running copy so its bundle isn't replaced underneath it.
osascript -e 'quit app "Just Notes"' >/dev/null 2>&1 || true

rm -rf "$dst"
ditto "$src" "$dst"
open "$dst"
echo "Installed and launched $app from $src."
