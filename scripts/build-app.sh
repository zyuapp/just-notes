#!/bin/sh
set -eu

# Builds the local app bundle with a stable code-signing identity.
#
# macOS ties a microphone or system-audio grant to the code signature that was
# granted, so the ad-hoc identity in tauri.conf.json makes every build look like
# a different app and the permission prompts come back after each install.
# Signing with a certificate keeps one grant valid across builds.
#
# Set JUST_NOTES_SIGNING_IDENTITY to override the certificate that is picked.

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

identity=${JUST_NOTES_SIGNING_IDENTITY:-}

if [ -z "$identity" ]; then
  identity=$(
    security find-identity -v -p codesigning |
      sed -n 's/.*"\(Apple Development: [^"]*\)".*/\1/p' |
      head -1
  )
fi

if [ -z "$identity" ]; then
  echo "No Apple Development certificate found; building with the ad-hoc identity from tauri.conf.json." >&2
  echo "macOS will ask for microphone and system audio access again after every install." >&2
  exec bun tauri build "$@"
fi

echo "Signing with: $identity"
APPLE_SIGNING_IDENTITY="$identity"
export APPLE_SIGNING_IDENTITY
exec bun tauri build "$@"
