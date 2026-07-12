# Mac App Store release

The repository can build a sandboxed, signed `.pkg`, but it deliberately contains no Apple credentials, provisioning profile, developer identity, or hosted-policy URL.

## Apple account setup

Before packaging a release:

1. Register an explicit macOS App ID whose bundle ID exactly matches `identifier` in `src-tauri/tauri.conf.json`. Change that identifier to a stable domain owned by the publisher before the first release if `dev.just-notes` is not the final registered ID.
2. Enable the required capabilities for the App ID and generate a **Mac App Store Connect** provisioning profile.
3. Install a **Mac App Distribution** signing identity and a **Mac Installer Distribution** signing identity, including their private keys, in the build Mac's keychain.
4. Create the macOS app record in App Store Connect using the same bundle ID.
5. Publish the privacy policy at a stable HTTPS URL and add that URL and a support URL to App Store Connect. Ensure the in-app Privacy & Legal screen points to the same policy.

Profiles, certificates, `.p8` API keys, passwords, and generated entitlements must never be committed. The packaging script copies temporary material under the ignored `src-tauri/.appstore/` directory.

## Package

Set these environment variables:

- `APPLE_TEAM_ID`: Apple Developer Team ID.
- `APPLE_APP_ID_PREFIX`: the App ID prefix shown for the registered identifier. It is often, but not always, the Team ID.
- `MAC_APP_STORE_PROFILE`: path to the downloaded `.provisionprofile`.
- `MAC_APP_DISTRIBUTION_IDENTITY`: exact keychain name of the Mac App Distribution certificate.
- `MAC_INSTALLER_DISTRIBUTION_IDENTITY`: exact keychain name of the Mac Installer Distribution certificate.
- `MAC_APP_BUILD_NUMBER`: a new positive integer for every upload.
- `MAC_APP_COPYRIGHT`: the publisher's complete copyright string.
- `VITE_PRIVACY_POLICY_URL`: stable HTTPS URL for the policy linked inside the app and in App Store Connect.
- `VITE_SUPPORT_URL`: stable HTTPS support page linked inside the app and in App Store Connect.

Optional variables:

- `MAC_APP_STORE_TARGET`: a Rust target such as `universal-apple-darwin`. If omitted, the script creates a native-architecture build.
- `MAC_APP_STORE_OUTPUT_DIR`: output directory; defaults to `dist/app-store`.

Then run:

```sh
bun run app:store:package
```

The script checks that the profile matches the configured bundle ID and Team ID, builds with the Mac App Distribution identity, verifies the sandboxed app signature and privacy manifest, and creates an installer-signed `.pkg` using `productbuild`. It does not upload anything.

Users upgrading from an earlier unsandboxed build can choose **Settings → Storage → Import…** and select the hidden `~/.just-notes` folder. Import works only before recordings exist in the new container, never merges or overwrites data, and retains access to the selected folder only for the duration of the copy.

## App Store Connect completion

Before submission, complete the product description, category, screenshots, age rating, pricing, territories, support and privacy URLs, App Privacy answers, and review notes. The review notes should explain the user-confirmed model download, its displayed size, local audio processing, Calendar use, and where to exercise each permission-dependent feature.

Just Notes uses standard HTTPS through Rustls to download the transcription model and declares `ITSAppUsesNonExemptEncryption` as false. The publisher remains responsible for confirming the applicable export-compliance exemption and answering App Store Connect's encryption questions accurately.

Review the bundled `PrivacyInfo.xcprivacy`, privacy policy, model attribution, and third-party notices whenever data handling or dependencies change. Confirm that generated dependency notices and native ONNX Runtime notices match the exact artifacts in the release build.
