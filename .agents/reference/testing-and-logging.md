# Testing & Logging

## Logging (Tracing)
- We use the `tracing` framework alongside `tracing-subscriber`.
- When initializing `fmt::init()`, ensure the writer is directed exclusively to **`stderr`** rather than `stdout`. If logs leak over `stdout`, the stdio MCP JSON transport fails.
- Use explicit levels: `trace!` for noisy variables, `info!` for lifecycle states, `error!` for dropped connections.

## Automated Testing
- Prefer standard `#[cfg(test)]` unit tests for core logical boundary conditions (like vector chunking checks).
- When writing tests containing async code, explicitly tag with `#[tokio::test]`.
- For advanced integration checks, utilize `test-log` to capture any tracing outputs natively within the standard `cargo test` framework without breaking.
