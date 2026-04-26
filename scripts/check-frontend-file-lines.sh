#!/bin/sh
set -eu

max_lines=160
violations="$(
  find src \
    -path src/bindings -prune -o \
    -type f \( -name '*.ts' -o -name '*.tsx' -o -name '*.css' \) \
    -exec wc -l {} + |
    awk -v max="$max_lines" '$2 != "total" && $1 > max { print }'
)"

if [ -n "$violations" ]; then
  echo "$violations"
  echo "Frontend files must stay at or below ${max_lines} lines, excluding generated bindings."
  exit 1
fi
