//! Gemini CLI adapter.
//!
//! Wraps the `gemini` official CLI tool.
//! - Installation: `npm install -g @google/gemini-cli` (example)
//! - Invocation: `gemini run "<prompt>"`

use async_trait::async_trait;
use overclock_core::config::AgentConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::Task;
use tracing::{info, warn};

use crate::adapter_trait::{AgentAdapter, HealthStatus, QuotaInfo, TaskOutput};

/// Gemini CLI adapter.
pub struct GeminiAdapter;

impl GeminiAdapter {
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
        config.binary.as_deref().unwrap_or("gemini")
    }
}

#[async_trait]
impl AgentAdapter for GeminiAdapter {
    fn name(&self) -> &str {
        "Gemini CLI"
    }

    fn agent_type(&self) -> &str {
        "gemini-cli"
    }

    async fn health_check(&self) -> HealthStatus {
        match tokio::process::Command::new("gemini")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                info!("Gemini CLI found: {version}");
                HealthStatus::Ready { version }
            }
            Ok(output) => {
                let reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
                warn!("Gemini CLI error: {reason}");
                HealthStatus::Error { reason }
            }
            Err(e) => {
                warn!("Gemini CLI not found: {e}");
                HealthStatus::NotInstalled {
                    reason: format!(
                        "CLI not found. Follow installation instructions for Gemini CLI."
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

        info!("Executing task '{}' with Gemini CLI", task.title);

        // Assuming `gemini run` is the command structure
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
                summary: format!("Gemini CLI failed: {stderr}"),
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

    async fn quota_info(&self, _config: &AgentConfig) -> anyhow::Result<Option<QuotaInfo>> {
        Ok(None)
    }
}
