# Just Notes Docs

This folder captures architecture notes for the current MVP codebase. The goal is to make the backend boundaries explicit enough that future changes can land in the right context without growing `src-tauri/src/lib.rs` again.

Start with:

- [Backend Architecture](backend-architecture.md) for the DDD-style bounded contexts and dependency direction.
- [Backend Module Map](backend-module-map.md) for the purpose of each Rust module and where common changes should go.

