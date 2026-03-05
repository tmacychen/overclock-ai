//! Role definitions and role-agent bindings.
//!
//! Roles define the *type* of work (architect, reviewer, developer, tester).
//! Each role is bound to a specific AI CLI agent in the project config,
//! determining which CLI tool executes tasks of that role.

use serde::{Deserialize, Serialize};

/// Built-in role kinds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RoleKind {
    /// Architecture design, technical research, project structure.
    Architect,
    /// Code review, design review, security audit.
    Reviewer,
    /// Code implementation.
    Developer,
    /// Test writing and execution.
    Tester,
    /// Deployment, CI/CD, infrastructure.
    DevOps,
    /// User-defined custom role.
    Custom(String),
}

impl std::fmt::Display for RoleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleKind::Architect => write!(f, "architect"),
            RoleKind::Reviewer => write!(f, "reviewer"),
            RoleKind::Developer => write!(f, "developer"),
            RoleKind::Tester => write!(f, "tester"),
            RoleKind::DevOps => write!(f, "devops"),
            RoleKind::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl RoleKind {
    /// Parse a role kind from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "architect" => RoleKind::Architect,
            "reviewer" => RoleKind::Reviewer,
            "developer" => RoleKind::Developer,
            "tester" => RoleKind::Tester,
            "devops" => RoleKind::DevOps,
            other => RoleKind::Custom(other.to_string()),
        }
    }
}

/// Configuration for a role, binding it to a default agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    /// Description of what this role does.
    pub description: String,
    /// Default agent ID to use for this role.
    pub default_agent: String,
    /// Optional prompt template file path (relative to templates/).
    pub prompt_template: Option<String>,
}
