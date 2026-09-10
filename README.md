# Just Notes

A local-first macOS notes app that records audio (microphone + system loopback) and transcribes it on-device, organizing everything into searchable note threads. Built with Tauri (Rust) and React/TypeScript. Transcription runs locally via Parakeet on sherpa-onnx — no audio leaves your machine.

## Requirements

- macOS 14.4+
- [Rust](https://rustup.rs) and [Bun](https://bun.sh)

## Develop

```sh
bun install
bun tauri dev      # run the app with hot reload
bun run verify     # build, lint, and test (frontend + Rust)
```

## Build

```sh
bun run app:build      # produce a .app bundle
bun run app:install    # build and install to /Applications
bun run app:release    # Developer ID-sign, notarize, and archive a direct release
```

## Docs

See [`docs/`](docs/README.md) for backend architecture and the module map.

## Release

Run the [release workflow](.github/workflows/release.yml) from the Actions tab
with a patch/minor/major bump. It bumps the version in `package.json`,
`src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`, builds on macOS, signs
with Developer ID, notarizes, then pushes the release commit and tag and
attaches the zip archive and dmg installer to a GitHub release.

## License

MIT — see [`LICENSE`](LICENSE). Third-party components ship their own notices in
`src-tauri/resources/ThirdPartyNotices.txt`; the Parakeet speech model is
downloaded at runtime under CC BY 4.0 (see `ModelAttribution.txt`).
