#!/bin/sh
set -eu

snapshot=$(mktemp -d)
trap 'rm -rf "$snapshot"' EXIT
cp -R src/bindings "$snapshot/bindings"

bun run bindings:rs

if ! diff -qr "$snapshot/bindings" src/bindings; then
  echo "Generated bindings are out of date. Run bun run bindings:rs and commit the result."
  exit 1
fi
