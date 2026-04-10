# Rust & Tokio Best Practices

## Error Handling
- Use `anyhow::Result` for all application-level errors and rapid prototyping.
- Do not blindly unwrap `Result` or `Option` types. Prefer propagating errors via `?`.
- For library-level structs expected to be used publicly, switch to `thiserror` for explicitly defined variants.

## Async Runtime
- Forge-MCP uses `tokio`. Ensure all I/O boundary code is completely asynchronous without blocking the master threads.
- For computationally heavy constraints (like local embeddings running on CPU via `fastembed-rs`), ALWAYS consider utilizing `tokio::task::spawn_blocking`.
