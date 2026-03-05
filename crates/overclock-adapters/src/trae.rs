//! Trae Agent CLI adapter (ByteDance).
//!
//! Wraps the `trae-cli` open-source tool.
//! - Installation: from GitHub (open-source, MIT license)
//! - Free tier: fully free (requires LLM API key)
//! - Invocation: `trae-cli run "<prompt>"`

use async_trait::async_trait;
use overclock_core::config::AgentConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::Task;
use tracing::{info, warn};

use crate::adapter_trait::{AgentAdapter, HealthStatus, TaskOutput};

/// Trae Agent CLI adapter.
pub struct TraeAdapter;

impl TraeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn build_prompt(task: &Task, context: &SharedContext) -> String {
        let ctx = context.to_prompt_context();
        format!(
            "{ctx}\n\n---\n\n# Current Task: {}\n\n{}\n\n\
            Execute this task completely. Report what you did and any files modified.",
            task.title, task.description
        )
    }

    fn binary(config: &AgentConfig) -> &str {
        config.binary.as_deref().unwrap_or("trae-cli")
    }
}

#[async_trait]
impl AgentAdapter for TraeAdapter {
    fn name(&self) -> &str {
        "Trae Agent"
    }

    fn agent_type(&self) -> &str {
        "trae-agent"
    }

    async fn health_check(&self) -> HealthStatus {
        match tokio::process::Command::new("trae-cli")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                info!("Trae CLI found: {version}");
                HealthStatus::Ready { version }
            }
            Ok(output) => {
                let reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
                warn!("Trae CLI error: {reason}");
                HealthStatus::Error { reason }
            }
            Err(e) => {
                warn!("Trae CLI not found: {e}");
                HealthStatus::NotInstalled {
                    reason: format!(
                        "CLI not found. Install from: https://github.com/anthropics/trae-agent"
                    ),
                }
            }
        }
    }

    async fn execute_task(
        &self,
        task: &Task,
        context: &SharedContext,
        config: &AgentConfig,
    ) -> anyhow::Result<TaskOutput> {
        let prompt = Self::build_prompt(task, context);
        let binary = Self::binary(config);

        info!("Executing task '{}' with Trae Agent", task.title);

        // Use trae-cli run for single-shot task execution.
        let output = tokio::process::Command::new(binary)
            .arg("run")
            .arg(&prompt)
            .current_dir(&context.workspace_root)
            .output()
            .await?;

        let raw_output = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Ok(TaskOutput {
                success: false,
                summary: format!("Trae Agent failed: {stderr}"),
                modified_files: vec![],
                artifacts: vec![],
                raw_output: format!("STDOUT:\n{raw_output}\n\nSTDERR:\n{stderr}"),
                exit_code: output.status.code(),
            });
        }

        Ok(TaskOutput {
            success: true,
            summary: raw_output.lines().take(5).collect::<Vec<_>>().join("\n"),
            modified_files: vec![],
            artifacts: vec![],
            raw_output,
            exit_code: output.status.code(),
        })
    }
}
