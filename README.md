# Overclock-AI

🚀 **Multi-AI Agent CLI Orchestration Platform**

Overclock-AI coordinates multiple AI CLI tools (CodeBuddy, Kiro CLI, Trae Agent, etc.) to work collaboratively on development tasks. Each agent operates in isolation, receiving tasks and context from the central orchestrator.

## Key Features

- **Role-based task assignment**: Architect, Reviewer, Developer, Tester
- **Multi-CLI orchestration**: CodeBuddy, Kiro CLI, Gemini CLI (free tier first!)
- **Shared context broker**: Agents are isolated but share curated context
- **DAG workflow engine**: Design → Review → Develop → Test pipelines
- **Extensible adapter system**: Add new AI CLI tools via plugin trait
- **Real-time TUI monitor**: Interactive terminal interface for tracking task execution and system events

## Design Principle

```
                     ┌─────────────────┐
                     │  Orchestrator   │
                     │ (overclock-ai)  │
                     └───────┬─────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
    ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐
    │  CodeBuddy  │  │  Kiro CLI   │  │ Gemini CLI  │
    │  (isolated) │  │  (isolated) │  │  (isolated) │
    └─────────────┘  └─────────────┘  └─────────────┘
```

Each CLI agent:
- Receives **only its own task** + shared context
- Has **no knowledge** of other agents
- Returns output to the orchestrator
- The orchestrator curates context for downstream tasks

## Quick Start

```bash
# Build
cargo build --workspace

# Initialize a project
overclock-ai init

# Check agent availability
overclock-ai status

# Create and run tasks
overclock-ai task create "Design the project architecture" --role architect
overclock-ai task list
overclock-ai run <task-id>

# Run a full workflow
overclock-ai run design-review-develop --workflow

# Start the TUI monitor for real-time task tracking
overclock-ai monitor
```

## Configuration

Edit `overclock-ai.toml` to configure agents and roles:

```toml
[agents.codebuddy]
type = "codebuddy"
# Path to the CLI binary (to avoid conflict with CodeBuddy IDE)
binary = "/opt/homebrew/lib/node_modules/@tencent-ai/codebuddy-code/bin/codebuddy"
free_tier = true

[agents.kiro]
type = "kiro-cli"
binary = "kiro-cli"
free_tier = true

[roles.architect]
default_agent = "codebuddy"
description = "Architecture design"
```

## AI CLI Installation

### CodeBuddy CLI
Install via npm:
```bash
npm install -g @tencent-ai/codebuddy-code
```
Note: If you have the CodeBuddy IDE installed, the `codebuddy` command might conflict. Use the full path in `overclock-ai.toml`: `/opt/homebrew/lib/node_modules/@tencent-ai/codebuddy-code/bin/codebuddy`.

### Kiro CLI
Install via curl:
```bash
curl -fsSL https://cli.kiro.dev/install | bash
```

### Gemini CLI
Install via npm:
```bash
npm install -g @google/gemini-cli
```

```
crates/
├── overclock-core/     # Core engine (task, role, workflow, context, event, telemetry, recovery)
├── overclock-adapters/ # AI CLI adapters (codebuddy, kiro, gemini)
├── overclock-cli/      # CLI entry point
└── overclock-server/   # REST API (Phase 2)
```

## License

MIT
