//! Shared context management.
//!
//! The Context Broker is the key mechanism that enables coordination between
//! isolated AI CLI agents. Each agent:
//!
//! 1. **Before execution**: Receives the shared context (project brief, previous
//!    task results, architecture decisions, code conventions)
//! 2. **After execution**: Its output is parsed and merged back into the shared context
//!
//! Agents never see each other's raw sessions — they only see the
//! orchestrator-curated shared context. This ensures clean handoffs.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Represents the shared context that flows between all tasks.
/// Stored in `.overclock-ai/context.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedContext {
    /// The project's root workspace path.
    pub workspace_root: PathBuf,
    /// Brief description of the project.
    pub project_brief: String,
    /// Key architecture decisions made so far.
    pub architecture_decisions: Vec<Decision>,
    /// Code conventions and standards.
    pub code_conventions: Vec<String>,
    /// Summary of completed task results (fed to downstream agents).
    pub task_results: Vec<TaskResultSummary>,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// An architecture decision record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub decided_at: DateTime<Utc>,
}

/// A summary of a completed task's result, used as context for downstream tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultSummary {
    pub task_title: String,
    pub role: String,
    pub agent_id: String,
    pub summary: String,
    pub artifact_paths: Vec<PathBuf>,
    pub completed_at: DateTime<Utc>,
}

/// The context directory name used within project workspaces.
pub const CONTEXT_DIR: &str = ".overclock-ai";
/// The context JSON filename.
pub const CONTEXT_FILE: &str = "context.json";

impl SharedContext {
    /// Create a new empty context for a workspace.
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            project_brief: String::new(),
            architecture_decisions: Vec::new(),
            code_conventions: Vec::new(),
            task_results: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    /// Compute the path to the `.overclock-ai/` directory.
    pub fn context_dir(&self) -> PathBuf {
        self.workspace_root.join(CONTEXT_DIR)
    }

    /// Load context from the `.overclock-ai/context.json` file.
    pub fn load(workspace_root: &Path) -> Result<Self> {
        let path = workspace_root.join(CONTEXT_DIR).join(CONTEXT_FILE);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let ctx: SharedContext = serde_json::from_str(&content)?;
            Ok(ctx)
        } else {
            Ok(Self::new(workspace_root.to_path_buf()))
        }
    }

    /// Save context to the `.overclock-ai/context.json` file.
    pub fn save(&mut self) -> Result<()> {
        self.updated_at = Utc::now();
        let dir = self.context_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(CONTEXT_FILE);
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a completed task result summary to the context.
    pub fn add_task_result(&mut self, summary: TaskResultSummary) {
        self.task_results.push(summary);
    }

    /// Build a prompt context string that can be injected into any agent.
    /// This is the unified context representation that all adapters use.
    pub fn to_prompt_context(&self) -> String {
        let mut parts = Vec::new();

        parts.push(format!("# Project Context\n\n{}", self.project_brief));

        if !self.architecture_decisions.is_empty() {
            parts.push("\n## Architecture Decisions".to_string());
            for d in &self.architecture_decisions {
                parts.push(format!("### {}\n{}\n**Rationale**: {}", d.title, d.description, d.rationale));
            }
        }

        if !self.code_conventions.is_empty() {
            parts.push("\n## Code Conventions".to_string());
            for c in &self.code_conventions {
                parts.push(format!("- {c}"));
            }
        }

        if !self.task_results.is_empty() {
            parts.push("\n## Previous Task Results".to_string());
            for r in &self.task_results {
                parts.push(format!(
                    "### {} (by {} using {})\n{}",
                    r.task_title, r.role, r.agent_id, r.summary
                ));
                if !r.artifact_paths.is_empty() {
                    parts.push("Artifacts:".to_string());
                    for a in &r.artifact_paths {
                        parts.push(format!("- {}", a.display()));
                    }
                }
            }
        }

        parts.join("\n")
    }
}
