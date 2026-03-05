//! Custom API adapter for user-purchased models.
//!
//! Allows direct API calls to OpenAI, Anthropic, Google, etc.
//! using the user's own API keys. This is a fallback for when
//! no free-tier CLI tool is suitable.

use async_trait::async_trait;
use overclock_core::config::AgentConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::Task;
use tracing::info;

use crate::adapter_trait::{AgentAdapter, HealthStatus, TaskOutput};

/// Custom API adapter — placeholder for direct model API integration.
pub struct CustomApiAdapter;

impl CustomApiAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AgentAdapter for CustomApiAdapter {
    fn name(&self) -> &str {
        "Custom API"
    }

    fn agent_type(&self) -> &str {
        "custom-api"
    }

    async fn health_check(&self) -> HealthStatus {
        // Custom API adapters need an API key env var to be set.
        HealthStatus::Ready {
            version: "custom-api v0.1".to_string(),
        }
    }

    async fn execute_task(
        &self,
        _task: &Task,
        _context: &SharedContext,
        config: &AgentConfig,
    ) -> anyhow::Result<TaskOutput> {
        info!(
            "Custom API adapter for provider '{}' — not yet implemented",
            config.provider.as_deref().unwrap_or("unknown")
        );

        // TODO: Implement direct HTTP API calls to model providers.
        Ok(TaskOutput {
            success: false,
            summary: format!(
                "Custom API adapter (provider: {}) is not yet implemented. \
                 Configure a CLI agent instead.",
                config.provider.as_deref().unwrap_or("unknown")
            ),
            modified_files: vec![],
            artifacts: vec![],
            raw_output: String::new(),
            exit_code: None,
        })
    }
}
