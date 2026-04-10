# Forge-MCP Orchestrator - Product Requirements Document

## 1. Executive Summary

Forge-MCP is a high-performance, Rust-native Model Context Protocol (MCP) server designed to equip AI agents with dynamic logic and long-term vector memory. Instead of building a standalone LLM interface, Forge-MCP acts as a universal local gateway, plugging directly into the Google Antigravity IDE. 

The core value proposition is **decoupled orchestration**: it provides Antigravity's Gemini agents with the ability to instantly recall massive datasets (RAG) and execute specialized Standard Operating Procedures (Agent MD Skills) locally, reducing token costs and completely bypassing the latency of Python-based frameworks.

**Current State:** Ground 0 (Greenfield Development).

---

## 2. Mission

**Mission Statement:** Build the fastest, most resource-efficient local MCP gateway to supercharge IDE-based agents with private memory and modular skills.

### Core Principles

1.  **Antigravity-First Integration** — Designed to plug natively into Google Antigravity via `stdio` for immediate, zero-friction usage.
2.  **Ultra-Low Latency** — Built in Rust to ensure tool-call routing overhead remains under 50ms.
3.  **Local Sovereign Memory** — All embeddings and database records remain strictly on the user's local machine via PostgreSQL.
4.  **Logic as Content** — Agent behaviors are defined in plain `.md` files, separating business logic from the compiled Rust binary.

---

## 3. Target Users

### Primary Persona: The Agent Architect

-   **Who:** Senior developers and system architects building multi-agent workflows.
-   **Goals:**
    -   Give their Antigravity agents persistent, long-term memory across sessions.
    -   Standardize agent behavior using version-controlled Markdown files.
-   **Pain Points:**
    -   Cloud-based RAG pipelines are expensive and slow.
    -   Context windows get cluttered when passing raw data instead of performing semantic searches.

---

## 4. System Architecture

### High-Level Architecture
┌─────────────────┐      stdio via  ┌─────────────────┐      SQL        ┌─────────────────┐
│  Google         │   mcp_config    │    Forge-MCP    │ ◄─────────────► │   PostgreSQL    │
│  Antigravity    │ ◄─────────────► │    (Rust Core)  │                 │   (Port 5432)   │
│  (IDE & LLM)    │                 │                 │                 │   w/ pgvector   │
└─────────────────┘                 └────────┬────────┘                 └─────────────────┘
│
┌───────▼────────┐
│ Agent MD Parser│
│ (File Watcher) │
└───────┬────────┘
│
┌───────▼────────┐
│ Local Inference│
│ (fastembed-rs) │
└────────────────┘

### Key Components

1.  **Forge-MCP Server Core (`main.rs`)**
    -   Implements the official `mcp-rust-sdk`.
    -   Manages the `stdio` connection lifecycle with the Antigravity client.
    -   Routes tool calls asynchronously using the `tokio` runtime.

2.  **Database (PostgreSQL)**
    -   Leverages the `pgvector` extension for high-performance semantic search.
    -   Stores embedded text chunks, associated metadata, and long-term agent memory.

3.  **Analysis & Logic Engine**
    -   **Embeddings:** `fastembed-rs` runs locally on the CPU to convert text into vector embeddings instantly.
    -   **Skill Parser:** A background service that monitors a local `./skills` directory, parsing `.md` files to expose them as dynamic MCP tools.

---

## 5. Features

### 5.1 Local Context Memory (RAG)
-   **Operation:** Converts any text provided by the agent into vector embeddings.
-   **Efficiency:** Uses BGE-Micro or similar CPU-optimized models to generate embeddings locally without API calls.
-   **Visibility:** Exposes a seamless semantic search tool to the LLM to pull exact paragraphs rather than entire documents.

### 5.2 Dynamic Skill Injection
-   **SOP Loading:** Parses Markdown files containing rules, structures, or persona instructions.
-   **Hot Swapping:** Agents immediately recognize new skills added to the directory without restarting the Rust server.
-   **Portability:** Skills can be version-controlled in Git and shared across teams.

### 5.3 Antigravity Native Interface
-   **Connection:** Configured entirely through Antigravity's `mcp_config.json`.
-   **Compatibility:** Designed strictly to the JSON-RPC 2.0 MCP standard, meaning it can later be used with Claude Desktop or Cursor with zero code changes.

---

## 6. Technology Stack

### Backend
| Component | Technology | Role |
|-----------|------------|------|
| Framework | Rust / Tokio | High-concurrency core and async runtime |
| Database | PostgreSQL | Persistent storage + `pgvector` for semantic search |
| Protocol | `mcp-rust-sdk` | Model Context Protocol implementation |
| DB Driver | `sqlx` | Compile-time checked, async SQL queries |

### AI / Intelligence Layer
| Component | Technology | Role |
|-----------|------------|------|
| Embeddings| `fastembed-rs` | Local text-to-vector generation |
| LLM | Google Antigravity | Client-side orchestrator handling all reasoning |

### Client Environment
| Component | Technology | Role |
|-----------|------------|------|
| IDE | Antigravity | Main user interface and agent host |
| Transport | stdio | Standard input/output communication bridge |

---

## 7. API Specification

### Base Connection: `stdio` (via JSON-RPC / MCP Standard)

#### MCP Tools Exposed to Antigravity
-   **`save_to_memory`**: 
    -   *Inputs:* `content` (string), `tags` (array of strings).
    -   *Action:* Vectorizes the text via `fastembed-rs` and stores it in PostgreSQL.
-   **`search_memory`**: 
    -   *Inputs:* `query` (string), `limit` (integer).
    -   *Action:* Returns the top *N* most semantically similar text chunks from the local database.
-   **`list_agent_skills`**: 
    -   *Inputs:* None.
    -   *Action:* Returns an array of available Markdown SOPs currently loaded in the skills directory.
-   **`read_skill`**:
    -   *Inputs:* `skill_name` (string).
    -   *Action:* Returns the full Markdown text of a specific skill to guide the agent's next steps.