//! CodeBuddy CLI adapter (Tencent Cloud).
//!
//! Wraps the `codebuddy` CLI tool.
//! - Installation: `brew install Tencent-CodeBuddy/tap/codebuddy-code`
//! - Free tier: 100K tokens/month + 50 daily completions
//! - Invocation: `codebuddy --print <prompt>` or interactive mode

use async_trait::async_trait;
use overclock_core::config::AgentConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::Task;
use tracing::{info, warn};

use crate::adapter_trait::{AgentAdapter, HealthStatus, TaskOutput};

/// CodeBuddy CLI adapter.
pub struct CodeBuddyAdapter;

impl CodeBuddyAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Build the prompt string for CodeBuddy from task + shared context.
    fn build_prompt(task: &Task, context: &SharedContext) -> String {
        let ctx = context.to_prompt_context();
        format!(
            "{ctx}\n\n---\n\n# Current Task: {}\n\n{}\n\n\
            Please complete this task. Provide your output as a clear summary of what you did, \
            followed by any code or file changes.",
            task.title, task.description
        )
    }

    /// Get the binary name from config or use default.
    fn binary(config: &AgentConfig) -> &str {
        config.binary.as_deref().unwrap_or("codebuddy")
    }
}

#[async_trait]
impl AgentAdapter for CodeBuddyAdapter {
    fn name(&self) -> &str {
        "CodeBuddy"
    }

    fn agent_type(&self) -> &str {
        "codebuddy"
    }

    async fn health_check(&self) -> HealthStatus {
        match tokio::process::Command::new("codebuddy")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                info!("CodeBuddy CLI found: {version}");
                HealthStatus::Ready { version }
            }
            Ok(output) => {
                let reason = String::from_utf8_lossy(&output.stderr).trim().to_string();
                warn!("CodeBuddy CLI error: {reason}");
                HealthStatus::Error { reason }
            }
            Err(e) => {
                warn!("CodeBuddy CLI not found: {e}");
                HealthStatus::NotInstalled {
                    reason: format!(
                        "CLI not found. Install with: brew install Tencent-CodeBuddy/tap/codebuddy-code"
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

        info!("Executing task '{}' with CodeBuddy CLI", task.title);

        let output = tokio::process::Command::new(binary)
            .arg("--print")
            .arg(&prompt)
            .current_dir(&context.workspace_root)
            .output()
            .await?;

        let raw_output = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Ok(TaskOutput {
                success: false,
                summary: format!("CodeBuddy failed: {stderr}"),
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
