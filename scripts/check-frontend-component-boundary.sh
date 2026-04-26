#!/bin/sh
set -eu

matches="$(rg --line-number '(@tauri-apps/api|from ["'\'']\.\./api|from ["'\'']\.\./\.\./api)' src/components || true)"

if [ -n "$matches" ]; then
  echo "$matches"
  echo "Render components must not import app API or raw Tauri IPC."
  exit 1
fi
