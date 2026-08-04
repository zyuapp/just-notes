#!/bin/sh
set -eu

# Builds the direct-distribution app bundle with a stable code-signing identity.
#
# macOS ties a microphone or system-audio grant to the code signature that was
# granted, so the ad-hoc identity in tauri.conf.json makes every build look like
# a different app and the permission prompts come back after each install.
# Signing with a certificate keeps one grant valid across builds.
#
# Set JUST_NOTES_SIGNING_IDENTITY to override the certificate that is picked.
# Set JUST_NOTES_RELEASE=1 and JUST_NOTES_NOTARY_PROFILE to create a notarized
# release archive using a notarytool keychain profile.

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

identity=${JUST_NOTES_SIGNING_IDENTITY:-}
release=${JUST_NOTES_RELEASE:-0}

case "$release" in
  0|1) ;;
  *) echo "JUST_NOTES_RELEASE must be 0 or 1" >&2; exit 1 ;;
esac

if [ -z "$identity" ]; then
  certificate_kind="Apple Development"
  if [ "$release" = 1 ]; then
    certificate_kind="Developer ID Application"
  fi
  identity=$(
    security find-identity -v -p codesigning |
      sed -n "s/.*\"\($certificate_kind: [^\"]*\)\".*/\1/p" |
      head -1
  )
fi

if [ -z "$identity" ]; then
  if [ "$release" = 1 ]; then
    echo "No Developer ID Application certificate found. Set JUST_NOTES_SIGNING_IDENTITY." >&2
    exit 1
  fi
  echo "No Apple Development certificate found; building with the ad-hoc identity from tauri.conf.json." >&2
  echo "macOS will ask for microphone and system audio access again after every install." >&2
  exec bun tauri build "$@"
fi

if [ "$release" = 1 ]; then
  if [ "$#" -ne 0 ]; then
    echo "Direct releases do not accept additional build arguments" >&2
    exit 1
  fi
  if [ "${SHERPA_ONNX_LIB_DIR+x}" = x ]; then
    echo "SHERPA_ONNX_LIB_DIR is not allowed for direct releases; the pinned, checksummed archive is required" >&2
    exit 1
  fi
  case "$identity" in
    Developer\ ID\ Application:*) ;;
    *) echo "Direct releases must use a Developer ID Application identity: $identity" >&2; exit 1 ;;
  esac
  if [ -z "${JUST_NOTES_NOTARY_PROFILE:-}" ]; then
    echo "Missing required environment variable: JUST_NOTES_NOTARY_PROFILE" >&2
    exit 1
  fi
fi

echo "Signing with: $identity"
APPLE_SIGNING_IDENTITY="$identity"
export APPLE_SIGNING_IDENTITY

if [ "$release" = 0 ]; then
  exec bun tauri build --bundles app "$@"
fi

target=aarch64-apple-darwin
release_workspace=$(mktemp -d "${TMPDIR:-/tmp}/just-notes-release.XXXXXX")
final_staging_dir=
cleanup() {
  /bin/rm -rf "$release_workspace"
  if [ -n "$final_staging_dir" ]; then
    /bin/rm -rf "$final_staging_dir"
  fi
}
trap cleanup EXIT HUP INT TERM

release_target_dir="$release_workspace/target"
CARGO_TARGET_DIR="$release_target_dir"
export CARGO_TARGET_DIR
bun tauri build --bundles app --target "$target"
app="$release_target_dir/$target/release/bundle/macos/Just Notes.app"
if [ ! -d "$app" ]; then
  echo "App bundle not found after release build" >&2
  exit 1
fi

/usr/bin/codesign --verify --deep --strict --verbose=2 "$app"
if /usr/bin/codesign -d --entitlements :- "$app" 2>/dev/null |
  /usr/bin/grep -q 'com.apple.security.app-sandbox'; then
  echo "Direct release unexpectedly contains the App Sandbox entitlement" >&2
  exit 1
fi
if ! /usr/bin/codesign -dvvv "$app" 2>&1 | /usr/bin/grep -Eq 'flags=.*runtime'; then
  echo "Direct release is missing hardened runtime" >&2
  exit 1
fi

version=$(/usr/bin/plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")
output_dir=${JUST_NOTES_RELEASE_OUTPUT_DIR:-dist/direct}
archive="$output_dir/Just-Notes-$version.zip"
mkdir -p "$output_dir"
submission_archive="$release_workspace/Just-Notes-$version-submission.zip"
/usr/bin/ditto -c -k --keepParent "$app" "$submission_archive"
/usr/bin/xcrun notarytool submit "$submission_archive" \
  --keychain-profile "$JUST_NOTES_NOTARY_PROFILE" \
  --wait
/usr/bin/xcrun stapler staple "$app"
/usr/bin/xcrun stapler validate "$app"
/usr/sbin/spctl --assess --type execute --verbose=2 "$app"

final_staging_dir=$(mktemp -d "$output_dir/.just-notes-release.XXXXXX")
staged_archive="$final_staging_dir/Just-Notes-$version.zip"
/usr/bin/ditto -c -k --keepParent "$app" "$staged_archive"
/bin/mv -f "$staged_archive" "$archive"

echo "Created notarized direct release: $archive"
