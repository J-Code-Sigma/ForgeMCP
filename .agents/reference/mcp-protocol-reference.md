# MCP Protocol Layout Reference

## Standard I/O (stdio) Basics
- The core integration for Antigravity occurs via Standard I/O streams using `stdio`.
- Communication happens purely via JSON-RPC 2.0 payloads delimited by newlines.
- **CRITICAL WARNING:** `stdout` is strictly reserved for the JSON-RPC messages payload. Do not ever use `println!` for logging unless writing implicitly to `stderr`. Unintentional text over stdout breaks the entire JSON IPC bridge with the Client IDE.

## Core Capabilities
- The server must respond to a server `initialize` JSON-RPC handshake correctly, dictating all known capabilities.
- Forge-MCP needs to expose logical endpoints representing variables: RAG (`save_to_memory`, `search_memory`) and Dynamic Agents (`list_agent_skills`, `read_skill`).
