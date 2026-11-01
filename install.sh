#!/bin/sh
# Install the latest Just Notes release.
#
#   curl -fsSL https://raw.githubusercontent.com/zyuapp/just-notes/main/install.sh | sh
#
# Puts the notarized app in /Applications (or ~/Applications when that is not writable).
set -eu

REPO="zyuapp/just-notes"
DOWNLOAD="https://github.com/$REPO/releases/latest/download"
ASSET="JustNotes.zip"
APP="Just Notes.app"

fail() { printf '\nInstall failed: %s\n' "$1" >&2; exit 1; }
say() { printf '%s\n' "$1"; }

[ "$(uname -s)" = Darwin ] || fail "Just Notes runs on macOS only."
arch="$(uname -m)"
if [ "$arch" = x86_64 ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ]; then arch=arm64; fi
[ "$arch" = arm64 ] || fail "Just Notes needs a Mac with Apple silicon, and this Mac is $arch."
os_version="$(sw_vers -productVersion)"
major="$(printf '%s' "$os_version" | cut -d. -f1)"
minor="$(printf '%s' "$os_version" | cut -d. -f2)"
case "$minor" in '' | *[!0-9]*) minor=0 ;; esac
if [ "$major" -lt 14 ] || { [ "$major" -eq 14 ] && [ "$minor" -lt 4 ]; }; then
  fail "Just Notes needs macOS 14.4 or later, and this Mac has $os_version."
fi
command -v curl >/dev/null 2>&1 || fail "curl is required."

applications=/Applications
[ -w "$applications" ] || applications="$HOME/Applications"
mkdir -p "$applications"

# Stage inside the destination so the final move is a rename on the same filesystem.
staging="$applications/.just-notes-install.$$"
rm -rf "$staging"
mkdir "$staging"
trap 'rm -rf "$staging"' EXIT INT TERM

say "Downloading Just Notes."
curl -fL --retry 3 --progress-bar -o "$staging/$ASSET" "$DOWNLOAD/$ASSET" \
  || fail "The download did not finish."
expected="$(curl -fsSL "$DOWNLOAD/$ASSET.sha256" | cut -d' ' -f1)" \
  || fail "Could not read the checksum of the latest release."
[ -n "$expected" ] || fail "The checksum of the latest release is empty."
[ "$(shasum -a 256 "$staging/$ASSET" | cut -d' ' -f1)" = "$expected" ] \
  || fail "The download did not match its checksum. Try again."

if pgrep -f "/$APP/Contents/MacOS/" >/dev/null 2>&1; then
  fail "Just Notes is running. Quit it, then run this again."
fi
/usr/bin/ditto -x -k "$staging/$ASSET" "$staging/unpacked" || fail "The download could not be unpacked."
[ -d "$staging/unpacked/$APP" ] || fail "The download contained no app."
target="$applications/$APP"
rm -rf "$target"
mv "$staging/unpacked/$APP" "$target"
version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$target/Contents/Info.plist" 2>/dev/null || echo latest)"
say "Installed Just Notes $version in $target"

open -a "$target"
