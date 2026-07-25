#!/bin/sh
set -eu

matches="$(rg --line-number '@tauri-apps/api/' src --glob '!src/api/**' || true)"

if [ -n "$matches" ]; then
  echo "$matches"
  echo "Raw Tauri IPC imports must stay inside src/api."
  exit 1
fi
