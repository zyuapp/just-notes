# Agent Instructions
- Before changing backend code, read `docs/backend-architecture.md` and `docs/backend-module-map.md`.
- Always design backend changes with a domain in mind. Put new behavior in the owning context instead of adding business logic to `src-tauri/src/lib.rs`.
- If a change does not clearly belong to an existing domain, define the domain responsibility first, then choose the module boundary.
- Keep `src-tauri/src/lib.rs` as a thin Tauri adapter for command wiring, setup, and state registration.
- Preserve the current direction of dependencies between `app`, `ipc`, `threads`, `capture`, `transcription`, and `recording`.
