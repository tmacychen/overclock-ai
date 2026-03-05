//! Unified adapter trait for all AI CLI agents.
//!
//! Every CLI adapter implements this trait. The orchestrator only interacts
//! with agents through this interface, ensuring all agents are interchangeable.

use async_trait::async_trait;
use overclock_core::config::AgentConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::Task;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Health status of a CLI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// CLI is installed and ready.
    Ready { version: String },
    /// CLI is not installed or not found.
    NotInstalled { reason: String },
    /// CLI is installed but not authenticated.
    NotAuthenticated { reason: String },
    /// CLI has an error.
    Error { reason: String },
}

impl HealthStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, HealthStatus::Ready { .. })
    }
}

/// Structured output from a CLI agent's task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Whether the task was successful.
    pub success: bool,
    /// Summary of what was accomplished.
    pub summary: String,
    /// Files created or modified.
    pub modified_files: Vec<PathBuf>,
    /// Artifact files produced.
    pub artifacts: Vec<PathBuf>,
    /// Complete raw output from the CLI tool.
    pub raw_output: String,
    /// Exit code of the CLI process.
    pub exit_code: Option<i32>,
}

/// Quota / usage information for a CLI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    /// Remaining credits/tokens (if known).
    pub remaining: Option<u64>,
    /// Total credits/tokens in the current period.
    pub total: Option<u64>,
    /// Period description (e.g., "monthly").
    pub period: Option<String>,
}

/// Unified adapter trait that all AI CLI tool adapters must implement.
///
/// The orchestrator calls these methods to:
/// 1. Check if the CLI is available (`health_check`)
/// 2. Execute a task (`execute_task`)
/// 3. Optionally query quota info (`quota_info`)
///
/// Each adapter translates between the unified interface and CLI-specific
/// invocation patterns. The agent never sees other agents — it only receives
/// the task description and shared context.
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Human-readable name of this adapter.
    fn name(&self) -> &str;

    /// Agent type identifier (matches `AgentConfig.agent_type`).
    fn agent_type(&self) -> &str;

    /// Check if the CLI tool is installed, authenticated, and ready.
    async fn health_check(&self) -> HealthStatus;

    /// Execute a task using this CLI agent.
    ///
    /// The adapter is responsible for:
    /// 1. Building the CLI command with the task description
    /// 2. Injecting the shared context (converted to CLI-native format)
    /// 3. Running the CLI as a subprocess
    /// 4. Capturing and parsing the output
    ///
    /// # Arguments
    /// * `task` - The task to execute
    /// * `context` - Shared context from the orchestrator (upstream results, project info)
    /// * `config` - Agent-specific configuration
    async fn execute_task(
        &self,
        task: &Task,
        context: &SharedContext,
        config: &AgentConfig,
    ) -> anyhow::Result<TaskOutput>;

    /// Query quota / usage information (if the CLI supports it).
    async fn quota_info(&self, _config: &AgentConfig) -> anyhow::Result<Option<QuotaInfo>> {
        Ok(None)
    }
}
