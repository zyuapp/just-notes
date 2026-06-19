#!/bin/sh
set -eu

status=0
violations="$(mktemp "${TMPDIR:-/tmp}/just-notes-rust-boundaries.XXXXXX")"

# Match a forbidden module reached through `crate::`, whether written directly
# (`crate::settings::X`) or pulled out of a grouped import
# (`crate::{ app::AppPaths, settings::X }`). The optional brace section absorbs
# the rest of the group, including one level of nested `{ ... }`, so a forbidden
# module is caught no matter where it sits in the group.
crate_pattern() {
  modules="$1"
  printf 'crate::(\\{([^{}]|\\{[^{}]*\\})*?)?\\b(%s)\\b' "$modules"
}

run_check() {
  domain="$1"
  pattern="$2"
  message="$3"

  if [ -d "src-tauri/src/$domain" ]; then
    if rg -nU "$pattern" "src-tauri/src/$domain" >> "$violations.tmp"; then
      {
        echo "$message"
        cat "$violations.tmp"
        echo
      } >> "$violations"
    fi
    : > "$violations.tmp"
  fi
}

check_forbidden() {
  run_check "$1" "$(crate_pattern "$2")" "$3"
}

: > "$violations.tmp"

check_forbidden "app" "capture|commands|indicator|ipc|platform|recording|settings|threads|transcription|tray" \
  "app must stay foundational and must not depend on feature domains."

check_forbidden "threads" "capture|commands|indicator|ipc|platform|recording|settings|transcription|tray" \
  "threads must persist note-thread data without depending on other feature domains."

check_forbidden "capture" "commands|indicator|ipc|platform|recording|settings|threads|tray" \
  "capture must own audio input without depending on ipc, recording, threads, or shell modules."

check_forbidden "recording" "commands|platform" \
  "recording must not depend on adapter or shell modules."
run_check "recording" "crate::lib|super::super::lib" \
  "recording must not depend on the Tauri shell."

check_forbidden "transcription" "commands|indicator|platform|recording|settings|tray" \
  "transcription must not depend on orchestration or shell modules."

check_forbidden "ipc" "app|capture|commands|indicator|platform|recording|settings|tray" \
  "ipc DTOs must not depend on app, capture, or orchestration contexts."

check_forbidden "settings" "capture|commands|indicator|ipc|platform|recording|threads|transcription|tray" \
  "settings must only build on app paths."

check_forbidden "platform" "app|capture|commands|indicator|ipc|recording|settings|threads|transcription|tray" \
  "platform shell helpers must stay free of domain dependencies."

check_forbidden "tray" "app|capture|commands|indicator|ipc|platform|recording|settings|threads|transcription" \
  "tray must stay a thin menu bar adapter without domain dependencies."

check_forbidden "indicator" "app|capture|commands|ipc|platform|recording|settings|threads|transcription|tray" \
  "indicator must stay a thin floating-window adapter without domain dependencies."

rm -f "$violations.tmp"

if [ -s "$violations" ]; then
  cat "$violations"
  status=1
fi
rm -f "$violations"

exit "$status"
