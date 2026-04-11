mod db;
mod embeddings;
mod skills_engine;

use anyhow::Result;
use db::DbClient;
use embeddings::EmbeddingEngine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skills_engine::SkillsEngine;
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(Deserialize, Debug)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    id: Option<Value>,
    #[allow(dead_code)]
    params: Option<Value>,
}

#[derive(Serialize, Debug)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
struct AgentModelEntry {
    id: String,
    description: String,
}

#[derive(Deserialize, Debug, Clone)]
struct AgentEntry {
    command: String,
    args: Vec<String>,
    model_flag: String,
    default_model: Option<String>,
    models: Vec<AgentModelEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct AgentsConfig {
    agents: std::collections::HashMap<String, AgentEntry>,
}

async fn handle_request(req: McpRequest, engine: Arc<SkillsEngine>, db: Arc<DbClient>, embed_mutex: Arc<Mutex<EmbeddingEngine>>, agents_config: Arc<AgentsConfig>) -> McpResponse {
    let result = match req.method.as_str() {
        "initialize" => {
            let params = req.params.as_ref().and_then(|v| v.as_object());
            let protocol_version = params.and_then(|p| p.get("protocolVersion")).and_then(|v| v.as_str()).unwrap_or("2024-11-05");
            
            json!({
                "protocolVersion": protocol_version,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "forge-mcp",
                    "version": "0.1.0"
                }
            })
        }
        "tools/list" => {
            json!({
                "tools": [
                    {
                        "name": "save_to_memory",
                        "description": "Saves chunked memory into pgvector",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "content": { "type": "string" },
                                "tags": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["content"]
                        }
                    },
                    {
                        "name": "search_memory",
                        "description": "Searches vector memory for context",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "list_agent_skills",
                        "description": "Lists all available markdown skills",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "read_skill",
                        "description": "Returns the markdown content of a skill",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "skill_name": { "type": "string" }
                            },
                            "required": ["skill_name"]
                        }
                    },
                    {
                        "name": "spawn_agent",
                        "description": "Spawns a headless sub-agent with a skill and goal. Supports multiple agent backends.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "skill_name": { "type": "string", "description": "Name of the skill to load (from skills directory)" },
                                "goal": { "type": "string", "description": "The goal for the sub-agent to achieve" },
                                "agent_type": { "type": "string", "description": "Agent backend to use (e.g. gemini). Use list_models to see options. Defaults to gemini." },
                                "model": { "type": "string", "description": "Model to use within the agent. Use list_models to see available options per agent." }
                            },
                            "required": ["skill_name", "goal"]
                        }
                    },
                    {
                        "name": "list_models",
                        "description": "Lists available agent backends and their models for use with spawn_agent.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_type": { "type": "string", "description": "Filter by a specific agent type (e.g. gemini). Omit to list all agents." }
                            }
                        }
                    },
                    {
                        "name": "index_workspace",
                        "description": "Crawls files recursively into vectors.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "directory": { "type": "string" }
                            },
                            "required": ["directory"]
                        }
                    }
                ]
            })
        }
        "tools/call" => {
            let params = req.params.as_ref().and_then(|v| v.as_object());
            let name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.and_then(|p| p.get("arguments")).and_then(|a| a.as_object());

            match name {
                "list_agent_skills" => {
                    match engine.list_agent_skills() {
                        Ok(skills) => {
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string(&skills).unwrap_or_default()
                                }],
                                "isError": false
                            })
                        }
                        Err(e) => {
                            json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                        }
                    }
                }
                "read_skill" => {
                    let skill_name = arguments.and_then(|a| a.get("skill_name")).and_then(|s| s.as_str()).unwrap_or("");
                    match engine.read_skill(skill_name) {
                        Ok(content) => {
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": content
                                }],
                                "isError": false
                            })
                        }
                        Err(e) => {
                            json!({ "content": [{ "type": "text", "text": format!("Error: {}", e) }], "isError": true })
                        }
                    }
                }
                "save_to_memory" => {
                    let content = arguments.and_then(|a| a.get("content")).and_then(|s| s.as_str()).unwrap_or("");
                    let tags = arguments.and_then(|a| a.get("tags")).and_then(|t| t.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                        .unwrap_or_default();
                    
                    let mut embed = embed_mutex.lock().await;
                    match embed.embed_text(content) {
                        Ok(embedding) => {
                            match db.save_to_memory(content, tags, embedding).await {
                                Ok(_) => json!({ "content": [{ "type": "text", "text": "Successfully saved to pgvector." }], "isError": false }),
                                Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error saving to DB: {}", e) }], "isError": true })
                            }
                        }
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error generating embedding: {}", e) }], "isError": true })
                    }
                }
                "search_memory" => {
                    let limit = arguments.and_then(|a| a.get("limit")).and_then(|l| l.as_i64()).unwrap_or(5);
                    let query_str = arguments.and_then(|a| a.get("query")).and_then(|s| s.as_str()).unwrap_or("");
        
                    let mut embed = embed_mutex.lock().await;
                    match embed.embed_text(query_str) {
                        Ok(embedding) => {
                            match db.search_memory(embedding, limit).await {
                                Ok(results) => json!({ "content": [{ "type": "text", "text": serde_json::to_string(&results).unwrap_or_default() }], "isError": false }),
                                Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error querying DB: {}", e) }], "isError": true })
                            }
                        }
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Error generating embedding: {}", e) }], "isError": true })
                    }
                }
                "spawn_agent" => {
                    let skill_name = arguments.and_then(|a| a.get("skill_name")).and_then(|s| s.as_str()).unwrap_or("");
                    let goal = arguments.and_then(|a| a.get("goal")).and_then(|s| s.as_str()).unwrap_or("");
                    let agent_type = arguments.and_then(|a| a.get("agent_type")).and_then(|s| s.as_str()).unwrap_or("gemini");
                    let model = arguments.and_then(|a| a.get("model")).and_then(|s| s.as_str());
                    
                    // Look up the agent config
                    let agent = match agents_config.agents.get(agent_type) {
                        Some(a) => a,
                        None => {
                            let available: Vec<&String> = agents_config.agents.keys().collect();
                            return McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(json!({ "content": [{ "type": "text", "text": format!("Unknown agent_type '{}'. Available: {:?}", agent_type, available) }], "isError": true })),
                                error: None,
                            };
                        }
                    };
                    
                    let selected_model = model.map(|m| m.to_string()).or_else(|| agent.default_model.clone());
                    info!("Spawning subagent [{}] with skill {} (model: {:?}) to achieve goal: {}", agent_type, skill_name, selected_model, goal);
                    
                    // Read the skill content to inject as context
                    let skill_context = engine.read_skill(skill_name).unwrap_or_default();
                    let prompt = format!(
                        "You are an autonomous sub-agent. Follow these skill instructions:\n\n{}\n\nYour goal: {}",
                        skill_context, goal
                    );
                    
                    let mut cmd = tokio::process::Command::new(&agent.command);
                    for arg in &agent.args {
                        cmd.arg(arg);
                    }
                    cmd.arg(&prompt);
                    if let Some(ref m) = selected_model {
                        cmd.arg(&agent.model_flag).arg(m);
                    }
                    
                    match cmd.output().await {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            let combined = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);
                            json!({ "content": [{ "type": "text", "text": combined }], "isError": !output.status.success() })
                        }
                        Err(e) => json!({ "content": [{ "type": "text", "text": format!("Failed to spawn subagent process: {}", e) }], "isError": true })
                    }
                }
                "list_models" => {
                    let agent_type_filter = arguments.and_then(|a| a.get("agent_type")).and_then(|s| s.as_str());
                    
                    let mut result_map = serde_json::Map::new();
                    for (name, agent) in &agents_config.agents {
                        if let Some(filter) = agent_type_filter {
                            if name != filter { continue; }
                        }
                        let models: Vec<Value> = agent.models.iter().map(|m| {
                            json!({ "id": m.id, "description": m.description })
                        }).collect();
                        result_map.insert(name.clone(), json!({
                            "command": agent.command,
                            "default_model": agent.default_model,
                            "models": models
                        }));
                    }
                    
                    json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result_map).unwrap_or_default()
                        }],
                        "isError": false
                    })
                }
                "index_workspace" => {
                    let directory = arguments.and_then(|a| a.get("directory")).and_then(|s| s.as_str()).unwrap_or("./");
                    info!("Indexing workspace directory: {}", directory);
                    let mut indexed_count = 0;
                    
                    for entry in walkdir::WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if ext == "rs" || ext == "md" {
                                    if let Ok(content) = tokio::fs::read_to_string(path).await {
                                        let tags = vec![path.to_string_lossy().to_string(), "workspace-rag".to_string()];
                                        for chunk in content.chars().collect::<Vec<char>>().chunks(1000) {
                                            let chunk_str: String = chunk.iter().collect();
                                            let mut embed = embed_mutex.lock().await;
                                            if let Ok(embedding) = embed.embed_text(&chunk_str) {
                                                let _ = db.save_to_memory(&chunk_str, tags.clone(), embedding).await;
                                            }
                                        }
                                        indexed_count += 1;
                                    }
                                }
                            }
                        }
                    }
                    
                    json!({ "content": [{ "type": "text", "text": format!("Successfully chunked and indexed {} files into Semantic Memory RAG.", indexed_count) }], "isError": false })
                }
                _ => {
                    json!({ "content": [{ "type": "text", "text": format!("Tool {} not implemented yet.", name) }], "isError": true })
                }
            }
        }
        _ => {
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(json!({
                    "code": -32601,
                    "message": "Method not found"
                })),
            };
        }
    };

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id,
        result: Some(result),
        error: None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Tracing must log to stderr, not stdout.
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .init();

    info!("Starting Forge-MCP server over stdio");

    // Dynamically resolve the absolute repository path, bypassing IDE CWD bugs.
    // Path structure: /repo/target/debug/forge-mcp -> trace 3 directories up.
    let exe_path = std::env::current_exe().unwrap_or_default();
    let repo_root = exe_path.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).unwrap_or(std::path::Path::new("."));
    let skills_path = repo_root.join("skills").to_string_lossy().to_string();
    let agents_config_path = repo_root.join("config").join("agents.json");
    
    let agents_config = match std::fs::read_to_string(&agents_config_path) {
        Ok(content) => {
            match serde_json::from_str::<AgentsConfig>(&content) {
                Ok(config) => {
                    info!("Loaded agents config with {} agent type(s): {:?}", config.agents.len(), config.agents.keys().collect::<Vec<_>>());
                    Arc::new(config)
                }
                Err(e) => {
                    error!("Failed to parse agents.json: {}", e);
                    return Err(anyhow::anyhow!("Invalid agents.json: {}", e));
                }
            }
        }
        Err(e) => {
            error!("Failed to read agents config at {:?}: {}", agents_config_path, e);
            return Err(anyhow::anyhow!("Missing agents.json at {:?}: {}", agents_config_path, e));
        }
    };
    
    let engine = Arc::new(SkillsEngine::new(&skills_path));
    
    let embed = match EmbeddingEngine::new() {
        Ok(e) => {
            info!("Fastembed model loaded successfully.");
            Arc::new(Mutex::new(e))
        }
        Err(e) => {
            error!("Failed to initialize fastembed: {}", e);
            return Err(e);
        }
    };

    let db = match DbClient::new("postgres://forge:password@localhost:5454/forge_mcp").await {
        Ok(client) => {
            info!("Database bound successfully.");
            Arc::new(client)
        }
        Err(e) => {
            error!("Failed to start database client: {}. Execution stopping.", e);
            return Err(e);
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

    // Stdout writer task ensures responses are serialized and flushed one at a time.
    tokio::spawn(async move {
        use std::io::Write;
        while let Some(msg) = rx.recv().await {
            println!("{}", msg);
            if let Err(e) = io::stdout().flush() {
                error!("Failed to flush stdout: {}", e);
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer).await {
            Ok(0) => {
                info!("EOF received on stdio. Shutting down.");
                break;
            }
            Ok(_) => {
                let input = buffer.trim().to_string();
                if input.is_empty() {
                    continue;
                }

                info!("Received raw request: {}", input);
                let tx = tx.clone();
                let engine = engine.clone();
                let db = db.clone();
                let embed = embed.clone();
                let agents_config = agents_config.clone();

                tokio::spawn(async move {
                    match serde_json::from_str::<McpRequest>(&input) {
                        Ok(req) => {
                            let is_notification = req.id.is_none();
                            let response = handle_request(req, engine, db, embed, agents_config).await;
                            
                            // JSON-RPC 2.0 strictly requires NOT responding to notifications
                            if !is_notification {
                                if let Ok(out) = serde_json::to_string(&response) {
                                    let _ = tx.send(out).await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse JSON-RPC: {}", e);
                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": -32700,
                                    "message": "Parse error"
                                }
                            });
                            let _ = tx.send(error_response.to_string()).await;
                        }
                    }
                });
            }
            Err(e) => {
                error!("Error reading stdio: {}", e);
                break;
            }
        }
    }

    Ok(())
}
