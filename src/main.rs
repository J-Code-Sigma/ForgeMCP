mod db;
mod embeddings;
mod skills_engine;

use anyhow::Result;
use db::DbClient;
use embeddings::EmbeddingEngine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skills_engine::SkillsEngine;
use std::io::{self, BufRead};
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

async fn handle_request(req: McpRequest, engine: &SkillsEngine, db: &DbClient, embed: &mut EmbeddingEngine) -> McpResponse {
    let result = match req.method.as_str() {
        "initialize" => {
            json!({
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

    let engine = SkillsEngine::new("skills/");
    
    let mut embed = match EmbeddingEngine::new() {
        Ok(e) => {
            info!("Fastembed model loaded successfully.");
            e
        }
        Err(e) => {
            error!("Failed to initialize fastembed: {}", e);
            return Err(e);
        }
    };

    let db = match DbClient::new("postgres://forge:password@localhost:5454/forge_mcp").await {
        Ok(client) => {
            info!("Database bound successfully.");
            client
        }
        Err(e) => {
            error!("Failed to start database client: {}. Execution stopping.", e);
            return Err(e);
        }
    };

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => {
                info!("EOF received on stdio. Shutting down.");
                break;
            }
            Ok(_) => {
                let input = buffer.trim();
                if input.is_empty() {
                    continue;
                }

                info!("Received raw request: {}", input);

                match serde_json::from_str::<McpRequest>(input) {
                    Ok(req) => {
                        let response = handle_request(req, &engine, &db, &mut embed).await;
                        // Send correctly formatted JSON back over stdout
                        let out = serde_json::to_string(&response)?;
                        println!("{}", out);
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
                        println!("{}", error_response);
                    }
                }
            }
            Err(e) => {
                error!("Error reading stdio: {}", e);
                break;
            }
        }
    }

    Ok(())
}
