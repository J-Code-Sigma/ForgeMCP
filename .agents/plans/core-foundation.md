# Plan: Core Foundation (In Progress)
- [x] Create project skeleton (`Cargo.toml`)
- [x] Configure stdio routing in `src/main.rs`.
- [ ] Connect stdio loop to an actual JSON-RPC parsing library (like tower based MCP servers).
- [ ] Ensure stdout/stderr logic is robust so log spam doesn't poison the MCP response stream.
