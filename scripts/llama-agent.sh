#!/bin/bash
# Local Llama agent wrapper for Forge-MCP spawn_agent
# Sends a prompt to the local llama.cpp server and prints the response

PROMPT="$1"
HOST="${LLAMA_HOST:-http://localhost:11434}"

RESPONSE=$(curl -s "$HOST/v1/chat/completions" \
  -H "Content-Type: application/json" \
  -d "{
    \"messages\": [{\"role\": \"user\", \"content\": $(echo "$PROMPT" | jq -Rs .)}],
    \"temperature\": 0.7
  }" 2>&1)

# Check if curl failed
if [ $? -ne 0 ]; then
  echo "Error: Could not connect to llama.cpp at $HOST" >&2
  echo "$RESPONSE" >&2
  exit 1
fi

# Extract the content from the response
echo "$RESPONSE" | jq -r '.choices[0].message.content // "Error: No response from model"'
