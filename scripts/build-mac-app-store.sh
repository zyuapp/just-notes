#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

require_env() {
  name=$1
  eval "value=\${$name:-}"
  if [ -z "$value" ]; then
    echo "Missing required environment variable: $name" >&2
    exit 1
  fi
}

require_env APPLE_TEAM_ID
require_env APPLE_APP_ID_PREFIX
require_env MAC_APP_STORE_PROFILE
require_env MAC_APP_DISTRIBUTION_IDENTITY
require_env MAC_INSTALLER_DISTRIBUTION_IDENTITY
require_env MAC_APP_BUILD_NUMBER
require_env MAC_APP_COPYRIGHT
require_env VITE_PRIVACY_POLICY_URL
require_env VITE_SUPPORT_URL

if [ "${SHERPA_ONNX_LIB_DIR+x}" = x ]; then
  echo "SHERPA_ONNX_LIB_DIR is not allowed for Mac App Store releases; the release must use the pinned, checksummed native archive" >&2
  exit 1
fi

case "$APPLE_TEAM_ID" in
  *[!A-Za-z0-9]*) echo "APPLE_TEAM_ID must contain only letters and digits" >&2; exit 1 ;;
esac
case "$APPLE_APP_ID_PREFIX" in
  *[!A-Za-z0-9]*) echo "APPLE_APP_ID_PREFIX must contain only letters and digits" >&2; exit 1 ;;
esac
case "$MAC_APP_BUILD_NUMBER" in
  ''|*[!0-9]*) echo "MAC_APP_BUILD_NUMBER must be a positive integer" >&2; exit 1 ;;
  0) echo "MAC_APP_BUILD_NUMBER must be greater than zero" >&2; exit 1 ;;
esac
case "$VITE_PRIVACY_POLICY_URL" in
  https://*) ;;
  *) echo "VITE_PRIVACY_POLICY_URL must be an HTTPS URL" >&2; exit 1 ;;
esac
case "$VITE_SUPPORT_URL" in
  https://*) ;;
  *) echo "VITE_SUPPORT_URL must be an HTTPS URL" >&2; exit 1 ;;
esac

if [ ! -f "$MAC_APP_STORE_PROFILE" ]; then
  echo "Provisioning profile not found: $MAC_APP_STORE_PROFILE" >&2
  exit 1
fi

bundle_id=$(/usr/bin/plutil -extract identifier raw src-tauri/tauri.conf.json)
work_dir=src-tauri/.appstore
generated_config=src-tauri/tauri.appstore.generated.conf.json
profile="$work_dir/embedded.provisionprofile"
entitlements="$work_dir/Entitlements.plist"
output_dir=${MAC_APP_STORE_OUTPUT_DIR:-dist/app-store}
target=aarch64-apple-darwin
if [ -n "${MAC_APP_STORE_TARGET:-}" ] && [ "$MAC_APP_STORE_TARGET" != "$target" ]; then
  echo "This release bundles a verified arm64-only transcription runtime; MAC_APP_STORE_TARGET must be $target" >&2
  exit 1
fi

mkdir -p "$work_dir" "$output_dir"
/usr/bin/ditto "$MAC_APP_STORE_PROFILE" "$profile"
/usr/bin/ditto src-tauri/Entitlements.plist "$entitlements"
/usr/libexec/PlistBuddy -c \
  "Add :com.apple.application-identifier string ${APPLE_APP_ID_PREFIX}.${bundle_id}" \
  "$entitlements"
/usr/libexec/PlistBuddy -c \
  "Add :com.apple.developer.team-identifier string ${APPLE_TEAM_ID}" \
  "$entitlements"

/usr/bin/plutil -lint "$entitlements" >/dev/null
/usr/bin/ditto src-tauri/tauri.appstore.conf.json "$generated_config"
/usr/bin/plutil -replace bundle.copyright -string "$MAC_APP_COPYRIGHT" "$generated_config"
/usr/bin/plutil -replace bundle.macOS.bundleVersion -string "$MAC_APP_BUILD_NUMBER" "$generated_config"
/usr/bin/plutil -replace bundle.macOS.signingIdentity -string "$MAC_APP_DISTRIBUTION_IDENTITY" "$generated_config"

profile_plist="$work_dir/profile.plist"
/usr/bin/security cms -D -i "$profile" > "$profile_plist"
profile_app_id=$(/usr/bin/plutil -extract 'Entitlements.com\.apple\.application-identifier' raw "$profile_plist")
profile_team_id=$(/usr/bin/plutil -extract 'Entitlements.com\.apple\.developer\.team-identifier' raw "$profile_plist")
expected_app_id="${APPLE_APP_ID_PREFIX}.${bundle_id}"
if [ "$profile_app_id" != "$expected_app_id" ]; then
  echo "Provisioning profile App ID is $profile_app_id; expected $expected_app_id" >&2
  exit 1
fi
if [ "$profile_team_id" != "$APPLE_TEAM_ID" ]; then
  echo "Provisioning profile team is $profile_team_id; expected $APPLE_TEAM_ID" >&2
  exit 1
fi

set -- bun tauri build --bundles app --config "$generated_config"
set -- "$@" --target "$target"
target_dir="src-tauri/target/$target/release"
# Force the native build script to run. It verifies the cached archive and
# freshly extracts the libraries, binding the release to the legal inventory.
cargo clean --manifest-path src-tauri/Cargo.toml -p sherpa-onnx-sys --target "$target"
APPLE_SIGNING_IDENTITY="$MAC_APP_DISTRIBUTION_IDENTITY" "$@"

app="$target_dir/bundle/macos/Just Notes.app"
if [ ! -d "$app" ]; then
  echo "App bundle not found at $app" >&2
  exit 1
fi
if [ ! -f "$app/Contents/embedded.provisionprofile" ]; then
  echo "App bundle is missing embedded.provisionprofile" >&2
  exit 1
fi
if [ ! -f "$app/Contents/Resources/PrivacyInfo.xcprivacy" ]; then
  echo "App bundle is missing Contents/Resources/PrivacyInfo.xcprivacy" >&2
  exit 1
fi
notices="$app/Contents/Resources/Legal/ThirdPartyNotices.txt"
if [ ! -f "$notices" ]; then
  echo "App bundle is missing generated third-party notices" >&2
  exit 1
fi
if ! /usr/bin/cmp -s src-tauri/resources/ThirdPartyNotices.txt "$notices"; then
  echo "Bundled third-party notices do not match the generated release inventory" >&2
  exit 1
fi
if { /usr/bin/nm -gU "$app/Contents/MacOS/just-notes" 2>/dev/null; /usr/bin/strings "$app/Contents/MacOS/just-notes"; } | /usr/bin/grep -Eqi 'espeak(_|::|-ng)'; then
  echo "The release binary unexpectedly contains GPL-licensed espeak-ng code; perform a compatibility review before submission" >&2
  exit 1
fi
privacy_manifest="$app/Contents/Resources/PrivacyInfo.xcprivacy"
/usr/bin/plutil -lint "$privacy_manifest" >/dev/null
privacy_api=$(/usr/bin/plutil -extract 'NSPrivacyAccessedAPITypes.0.NSPrivacyAccessedAPIType' raw "$privacy_manifest")
privacy_reasons=$(/usr/bin/plutil -extract 'NSPrivacyAccessedAPITypes.0.NSPrivacyAccessedAPITypeReasons' json -o - "$privacy_manifest")
if [ "$privacy_api" != "NSPrivacyAccessedAPICategoryFileTimestamp" ]; then
  echo "Privacy manifest does not declare the file timestamp API category" >&2
  exit 1
fi
case "$privacy_reasons" in
  *'C617.1'*) ;;
  *) echo "Privacy manifest is missing the app-container file timestamp reason" >&2; exit 1 ;;
esac

/usr/bin/codesign --verify --deep --strict --verbose=2 "$app"
signed_entitlements="$work_dir/signed-entitlements.plist"
/usr/bin/codesign -d --entitlements :- "$app" > "$signed_entitlements"
signed_app_id=$(/usr/bin/plutil -extract 'com\.apple\.application-identifier' raw "$signed_entitlements")
signed_team_id=$(/usr/bin/plutil -extract 'com\.apple\.developer\.team-identifier' raw "$signed_entitlements")
sandbox=$(/usr/bin/plutil -extract 'com\.apple\.security\.app-sandbox' raw "$signed_entitlements")
if [ "$signed_app_id" != "$expected_app_id" ] || [ "$signed_team_id" != "$APPLE_TEAM_ID" ] || [ "$sandbox" != "true" ]; then
  echo "Signed app entitlements do not match the profile, team, and sandbox requirements" >&2
  exit 1
fi

version=$(/usr/bin/plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")
pkg="$output_dir/Just-Notes-${version}-${MAC_APP_BUILD_NUMBER}.pkg"
/usr/bin/xcrun productbuild \
  --sign "$MAC_INSTALLER_DISTRIBUTION_IDENTITY" \
  --component "$app" /Applications \
  "$pkg"
/usr/sbin/pkgutil --check-signature "$pkg"

echo "Created Mac App Store package: $pkg"
echo "Validate and upload it with Transporter or App Store Connect tooling after completing release metadata."
