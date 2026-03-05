//! Task data model and state machine.
//!
//! A Task is the fundamental unit of work in overclock-ai. Each task is assigned
//! to a role (e.g., Architect, Reviewer) and dispatched to a specific AI CLI agent.
//! Agents execute tasks in isolation — they don't know about other agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for a task.
pub type TaskId = Uuid;

/// Result produced by a completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Summary of what the agent accomplished.
    pub summary: String,
    /// Files created or modified by the agent.
    pub modified_files: Vec<PathBuf>,
    /// Artifact files produced (reports, designs, etc.).
    pub artifacts: Vec<PathBuf>,
    /// Raw output from the CLI agent.
    pub raw_output: String,
}

/// Status of a task through its lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum TaskStatus {
    /// Task is created but not yet assigned to an agent.
    Pending,
    /// Task is assigned to an agent and waiting to run.
    Assigned {
        agent_id: String,
        assigned_at: DateTime<Utc>,
    },
    /// Task is currently being executed by the agent.
    Running {
        agent_id: String,
        started_at: DateTime<Utc>,
    },
    /// Task is completed, waiting for review by another agent.
    AwaitingReview {
        reviewer_agent_id: String,
        result: TaskResult,
    },
    /// Task completed successfully.
    Completed {
        completed_at: DateTime<Utc>,
        result: TaskResult,
    },
    /// Task failed.
    Failed {
        failed_at: DateTime<Utc>,
        error: String,
    },
}

/// A task to be executed by an AI CLI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: TaskId,
    /// Human-readable title.
    pub title: String,
    /// Detailed description / prompt for the agent.
    pub description: String,
    /// Current status.
    pub status: TaskStatus,
    /// Role this task is assigned to.
    pub role: String,
    /// Specific agent ID (from config) to execute this task.
    /// If None, uses the role's default agent.
    pub agent_id: Option<String>,
    /// Tasks that must complete before this one can start.
    pub dependencies: Vec<TaskId>,
    /// Additional context to inject into the agent prompt.
    pub extra_context: Vec<String>,
    /// When the task was created.
    pub created_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task in Pending status.
    pub fn new(title: impl Into<String>, description: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            role: role.into(),
            agent_id: None,
            dependencies: Vec::new(),
            extra_context: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Check if all dependencies are satisfied given a set of completed task IDs.
    pub fn dependencies_satisfied(&self, completed: &[TaskId]) -> bool {
        self.dependencies.iter().all(|dep| completed.contains(dep))
    }

    /// Check if the task is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, TaskStatus::Completed { .. } | TaskStatus::Failed { .. })
    }
}
