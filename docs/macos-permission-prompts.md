# macOS Permission Prompts

Why microphone and system-audio prompts reappear during local development, and
what the build scripts do about it.

## How macOS decides who is asking

macOS records a permission grant against a *code identity*: the signing
identifier plus a code requirement derived from the signing certificate. A
request is matched against the stored requirement, and a request that fails to
match is treated as a first-time request from an unknown app.

An ad-hoc signature has no certificate, so its requirement pins the binary hash
instead. Every rebuild produces a new hash, which is why an ad-hoc app is a new
app to the permission system on each install:

```
tccd: Failed to match existing code requirement for subject <app>
tccd: AUTHREQ_PROMPTING: service=kTCCServiceMicrophone, subject=<app>
```

`tauri.conf.json` sets `bundle.macOS.signingIdentity` to `-` (ad-hoc), so every
`bun run app:install` used to re-ask for the microphone and for system audio.
`scripts/build-app.sh` signs with an Apple Development certificate instead,
which produces a requirement that survives rebuilds. One grant, kept.

Mac App Store packaging is unaffected: `scripts/build-mac-app-store.sh` runs
`tauri build` directly with its own distribution identity.

## `tauri dev` is attributed to the terminal

The dev build is a bare executable rather than an app bundle, so it has no
identity of its own to be attributed to. macOS holds the *launching* process
responsible and prompts on its behalf — the terminal, editor, or agent that ran
`bun run tauri dev`:

```
AUTHREQ_ATTRIBUTION: responsible={com.cmuxterm.app}, requesting={.../target/debug/just-notes}
AUTHREQ_SUBJECT:     subject=com.cmuxterm.app
AUTHREQ_RESULT:      authValue=2
```

Signing the dev binary does not change this. The grant belongs to the terminal
and is stable there, so a terminal that has already been allowed once stays
quiet; a different terminal asks once, on its own behalf.

## Tests never prompt

`bun run test:rs` and `bun run test:quality` read WAV fixtures and never open a
capture device, so they request nothing. A prompt during a test run comes from
another app, not from this suite.

The two entry points that do request access are:

- `capture/permission.rs` — `AVCaptureDevice::requestAccessForMediaType`, on record.
- `capture/system_loopback/tap.rs` — `AudioHardwareCreateProcessTap`. Core Audio
  exposes no way to read tap authorization, so `get_permissions_status` learns it
  by creating a tap. Reading permission state is itself a request.
