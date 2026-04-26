#!/bin/sh
set -eu

status=0
violations="$(mktemp "${TMPDIR:-/tmp}/just-notes-rust-boundaries.XXXXXX")"

check_forbidden() {
  domain="$1"
  forbidden="$2"
  message="$3"

  if [ -d "src-tauri/src/$domain" ]; then
    if rg -n "$forbidden" "src-tauri/src/$domain" >> "$violations.tmp"; then
      {
        echo "$message"
        cat "$violations.tmp"
        echo
      } >> "$violations"
    fi
    : > "$violations.tmp"
  fi
}

: > "$violations.tmp"

check_forbidden "app" "crate::(capture|ipc|recording|threads|transcription)" \
  "app must stay foundational and must not depend on feature domains."

check_forbidden "threads" "crate::(capture|ipc|recording|transcription)" \
  "threads must persist note-thread data without depending on capture, ipc, recording, or transcription."

check_forbidden "capture" "crate::(ipc|recording|threads)" \
  "capture must own audio input without depending on ipc, recording, or threads."

check_forbidden "recording" "crate::lib|super::super::lib" \
  "recording must not depend on the Tauri shell."

check_forbidden "transcription" "crate::recording" \
  "transcription must not depend on recording orchestration."

check_forbidden "ipc" "crate::(app|capture|recording)" \
  "ipc DTOs must not depend on app, capture, or recording contexts."

rm -f "$violations.tmp"

if [ -s "$violations" ]; then
  cat "$violations"
  status=1
fi
rm -f "$violations"

exit "$status"
