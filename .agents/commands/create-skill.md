---
description: Generate a new behavior SOP in the skills directory
---

# Create Skill SOP

## Purpose
Forge-MCP uses "Decoupled Orchestration". Logic is in `.md` files.

## Execution
Ask the user what standard operating procedure the agent requires. Write the logic constraints and instructions to `./skills/[name].md`. This file will be parsed by `src/skills_engine.rs`.
