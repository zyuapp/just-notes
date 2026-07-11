#!/bin/sh
# Builds the transcript-quality fixtures used by `bun run test:quality` from
# mini LibriSpeech (openslr.org/31). Downloads once into a local cache, then
# regenerates the fixture WAV pairs and reference transcripts.
set -eu

cache="${QUALITY_FIXTURES_CACHE:-$HOME/.just-notes/quality-fixtures-cache}"
out="${QUALITY_FIXTURES_DIR:-$HOME/.just-notes/quality-fixtures}"
url="https://www.openslr.org/resources/31/dev-clean-2.tar.gz"
corpus="$cache/LibriSpeech/dev-clean-2"

mkdir -p "$cache"
if [ ! -d "$corpus" ]; then
  archive="$cache/dev-clean-2.tar.gz"
  if [ ! -f "$archive" ]; then
    echo "Downloading mini LibriSpeech (~120 MB, one time)..."
    curl -L --fail -o "$archive" "$url"
  fi
  tar -xzf "$archive" -C "$cache"
fi

python3 "$(dirname "$0")/make_quality_fixtures.py" "$corpus" "$out"
