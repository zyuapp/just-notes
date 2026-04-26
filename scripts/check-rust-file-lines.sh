#!/bin/sh
set -eu

max_lines="${RUST_FILE_MAX_LINES:-500}"
baseline_file="${RUST_FILE_LINES_BASELINE:-scripts/rust-file-lines.baseline}"
status=0
violations="$(mktemp "${TMPDIR:-/tmp}/just-notes-rust-lines.XXXXXX")"

find src-tauri/src -name '*.rs' -print | while IFS= read -r path; do
  lines="$(wc -l < "$path" | tr -d ' ')"
  limit="$max_lines"

  if [ -f "$baseline_file" ]; then
    baseline_limit="$(
      awk -v target="$path" '$1 == target { print $2; found = 1 } END { if (!found) exit 1 }' "$baseline_file" || true
    )"
    if [ -n "$baseline_limit" ]; then
      limit="$baseline_limit"
    fi
  fi

  if [ "$lines" -gt "$limit" ]; then
    echo "$path has $lines lines; limit is $limit" >> "$violations"
  fi
done

if [ -s "$violations" ]; then
  cat "$violations"
  status=1
fi
rm -f "$violations"

exit "$status"
