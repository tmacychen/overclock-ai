//! Event system for real-time monitoring and UI updates.
//!
//! Events are emitted by the orchestrator and adapters during task execution.
//! The CLI's TUI monitor and future Web UI subscribe to these events.

use crate::task::TaskId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Events emitted during orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorEvent {
    /// A new task was created.
    TaskCreated {
        task_id: TaskId,
        title: String,
        role: String,
    },
    /// A task was assigned to an agent.
    TaskAssigned {
        task_id: TaskId,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    /// An agent started executing a task.
    AgentStarted {
        task_id: TaskId,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Incremental output from an agent.
    AgentOutput {
        task_id: TaskId,
        agent_id: String,
        content: String,
        timestamp: DateTime<Utc>,
    },
    /// An agent completed a task.
    TaskCompleted {
        task_id: TaskId,
        agent_id: String,
        summary: String,
        timestamp: DateTime<Utc>,
    },
    /// An agent failed on a task.
    TaskFailed {
        task_id: TaskId,
        agent_id: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// A workflow started.
    WorkflowStarted {
        workflow_name: String,
        total_steps: usize,
        timestamp: DateTime<Utc>,
    },
    /// A workflow completed.
    WorkflowCompleted {
        workflow_name: String,
        timestamp: DateTime<Utc>,
    },
}

/// Event bus backed by tokio broadcast channel.
pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<OrchestratorEvent>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: OrchestratorEvent) {
        // Ignore send errors (no active receivers).
        let _ = self.sender.send(event);
    }

    /// Subscribe to events. Returns a receiver.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<OrchestratorEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
