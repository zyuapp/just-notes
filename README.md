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
```

## Docs

See [`docs/`](docs/README.md) for backend architecture and the module map.
