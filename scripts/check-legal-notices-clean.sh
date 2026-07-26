#!/bin/sh
set -eu

snapshot=$(mktemp)
trap 'rm -f "$snapshot"' EXIT
cp src-tauri/resources/ThirdPartyNotices.txt "$snapshot"

bun run legal:notices

if ! cmp -s "$snapshot" src-tauri/resources/ThirdPartyNotices.txt; then
  echo "Generated third-party notices are out of date. Run bun run legal:notices and commit the result."
  exit 1
fi
