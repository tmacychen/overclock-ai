//! Project configuration management.
//!
//! Reads `overclock-ai.toml` from the project root, which defines:
//! - Available AI CLI agents and their settings
//! - Role-to-agent bindings
//! - Project metadata

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::role::RoleConfig;

/// Top-level project configuration (from `overclock-ai.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project metadata.
    pub project: ProjectMeta,
    /// Available AI CLI agents, keyed by agent ID.
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,
    /// Role definitions, keyed by role name.
    #[serde(default)]
    pub roles: HashMap<String, RoleConfig>,
}

/// Project metadata section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    /// Project name.
    pub name: String,
    /// Workspace root (relative or absolute).
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_workspace() -> String {
    ".".to_string()
}

/// Configuration for an AI CLI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent type identifier (e.g., "codebuddy", "kiro-cli", "trae-agent").
    #[serde(rename = "type")]
    pub agent_type: String,
    /// Binary name or path to the CLI executable.
    #[serde(default)]
    pub binary: Option<String>,
    /// Whether this agent uses a free tier.
    #[serde(default)]
    pub free_tier: bool,
    /// Default model to use (if the CLI supports model selection).
    #[serde(default)]
    pub default_model: Option<String>,
    /// API key environment variable name (for custom API agents).
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Provider name (for custom API agents: "openai", "anthropic", etc.).
    #[serde(default)]
    pub provider: Option<String>,
    /// Agent mode (e.g., "cli", "api", "ide-proxy").
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "cli".to_string()
}

/// Config file name.
pub const CONFIG_FILE: &str = "overclock-ai.toml";

impl ProjectConfig {
    /// Load configuration from `overclock-ai.toml` in the given directory.
    pub fn load(workspace_root: &Path) -> Result<Self> {
        let config_path = workspace_root.join(CONFIG_FILE);
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: ProjectConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            anyhow::bail!(
                "Configuration file not found: {}. Run `overclock-ai init` first.",
                config_path.display()
            );
        }
    }

    /// Save configuration to `overclock-ai.toml`.
    pub fn save(&self, workspace_root: &Path) -> Result<()> {
        let config_path = workspace_root.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Generate a default configuration.
    pub fn default_config(project_name: &str) -> Self {
        let mut agents = HashMap::new();
        agents.insert(
            "codebuddy".into(),
            AgentConfig {
                agent_type: "codebuddy".into(),
                // Use specific path to avoid conflict with CodeBuddy IDE
                binary: Some(
                    "/opt/homebrew/lib/node_modules/@tencent-ai/codebuddy-code/bin/codebuddy"
                        .into(),
                ),
                free_tier: true,
                default_model: None,
                api_key_env: None,
                provider: None,
                mode: "cli".into(),
            },
        );
        agents.insert(
            "kiro".into(),
            AgentConfig {
                agent_type: "kiro-cli".into(),
                binary: Some("kiro-cli".into()),
                free_tier: true,
                default_model: None,
                api_key_env: None,
                provider: None,
                mode: "cli".into(),
            },
        );
        agents.insert(
            "trae".into(),
            AgentConfig {
                agent_type: "trae-agent".into(),
                binary: Some("trae-cli".into()),
                free_tier: true,
                default_model: None,
                api_key_env: None,
                provider: None,
                mode: "cli".into(),
            },
        );

        let mut roles = HashMap::new();
        roles.insert(
            "architect".into(),
            RoleConfig {
                description: "项目架构设计和技术调研".into(),
                default_agent: "codebuddy".into(),
                prompt_template: Some("architect.md".into()),
            },
        );
        roles.insert(
            "reviewer".into(),
            RoleConfig {
                description: "审查代码和架构方案，提出改进建议".into(),
                default_agent: "kiro".into(),
                prompt_template: Some("reviewer.md".into()),
            },
        );
        roles.insert(
            "developer".into(),
            RoleConfig {
                description: "编码实现".into(),
                default_agent: "trae".into(),
                prompt_template: Some("developer.md".into()),
            },
        );
        roles.insert(
            "tester".into(),
            RoleConfig {
                description: "编写和执行测试".into(),
                default_agent: "codebuddy".into(),
                prompt_template: Some("tester.md".into()),
            },
        );

        ProjectConfig {
            project: ProjectMeta {
                name: project_name.to_string(),
                workspace: ".".to_string(),
            },
            agents,
            roles,
        }
    }
}
