#!/bin/sh
set -eu

bun run bindings:rs

changes="$(git status --porcelain src/bindings)"
if [ -n "$changes" ]; then
  echo "$changes"
  echo "Generated bindings are out of date. Run bun run bindings:rs and commit the result."
  exit 1
fi
