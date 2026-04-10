# 🚀 Forge-MCP

**Forge-MCP** is a high-performance, Rust-native Model Context Protocol (MCP) server designed as a local-first RAG and dynamic "Skill" injection gateway. It securely orchestrates local intelligence via dynamic filesystems and vector math without relying on expensive, high-latency cloud frameworks or Python dependencies.

It maps directly into IDE systems like **Google Antigravity** via the standard `stdio` MCP JSON-RPC protocol.

## 🧠 Core Architecture
Our entire pipeline achieves zero-latency local computation:
- **Rust/Tokio**: The core backbone ensuring safety and async concurrency.
- **FastEmbed-rs (`v5.13`)**: Natively evaluates `BGE-Small-EN-V1.5` ML weights via ONNX local execution. It requires **no API keys** and generates identical 384-dimensional arrays completely locally.
- **PgVector (`sqlx`)**: Persists the semantic vectors inside a local PostgreSQL instance, indexing via ultra-fast Hierarchical Navigable Small World (`HNSW`) cosine-distance operator math (`<=>`).
- **Dynamic Skills Engine**: Skips complex coding cycles. Forge loads its executable agent behaviors by simply reading `.md` Standard Operating Procedure (SOP) files right out of the `./skills/` directory dynamically.

## 🛠 Features (MCP Tools)
Forge-MCP currently supports standard MCP tools over `stdio`:
- `list_agent_skills`: Automatically discovers logic algorithms stored in the `/skills` directory.
- `read_skill`: Reads and parses specific markdown files dynamically for context injection.
- `save_to_memory`: Converts chunks of text into AI vectors via `fastembed` and statically bounds them to Postgres.
- `search_memory`: Accepts natural language query strings, vectorizes them, and performs HNSW index sorting traversing Postgres vectors simultaneously.

## ⚡ Quickstart

### 1. Initialize Postgres (PgVector)
Ensure Docker is installed on your machine. Our pipeline maps onto port `5454` explicitly to avoid system overlapping.
```bash
docker-compose up -d
```

### 2. Compile and Cache Machine Learning Weights
Compile the Rust binary natively. Because we load localized Artificial Intelligence logic, the extremely first boot will require roughly 30-45 seconds to download the cached ONNX model to your `~/.cache` directory.
```bash
cargo build
```

### 3. Connect as an active MCP
Use your IDE's agent settings to point it directly to the compiled rust executable:

```bash
# Example routing using Google Antigravity CLI
antigravity --add-mcp '{"name":"forge-mcp","command":"/absolute/path/to/Forge-MCP/target/debug/forge-mcp"}'
```

Test it instantly by asking your Agent: *"Can you list all of my specific Forge skills?"*

*Built for absolute system supremacy at Ground 0.*
