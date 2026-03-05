//! Workflow engine — DAG-based task orchestration.
//!
//! A Workflow defines a directed acyclic graph (DAG) of tasks that
//! the orchestrator executes in order. Tasks can run in parallel when
//! they have no dependencies on each other. The orchestrator ensures:
//!
//! 1. Each CLI agent receives only its own task + shared context
//! 2. Agent outputs are collected and fed into the shared context
//! 3. Downstream tasks receive upstream results as context
//! 4. Agents never communicate directly — all routing goes through the orchestrator

use serde::{Deserialize, Serialize};
use crate::task::{Task, TaskId};

/// A step in a workflow, referencing a task configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Unique name for this step within the workflow.
    pub name: String,
    /// Role to assign this step to (e.g., "architect", "reviewer").
    pub role: String,
    /// Task description / prompt.
    pub description: String,
    /// Specific agent override (if not using the role default).
    pub agent_id: Option<String>,
    /// Names of steps that must complete before this one.
    pub depends_on: Vec<String>,
}

/// A workflow definition — a named DAG of steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Workflow name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Ordered list of steps (topological order recommended).
    pub steps: Vec<WorkflowStep>,
}

/// Status of a running workflow instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    /// Workflow definition name.
    pub workflow_name: String,
    /// Mapping from step name to generated task ID.
    pub step_tasks: Vec<(String, TaskId)>,
    /// Whether the workflow is still running.
    pub completed: bool,
}

impl Workflow {
    /// Generate tasks from this workflow definition.
    /// Returns tasks in dependency-respecting order.
    pub fn generate_tasks(&self) -> Vec<Task> {
        let mut tasks: Vec<Task> = Vec::new();
        let mut name_to_id: std::collections::HashMap<String, TaskId> = std::collections::HashMap::new();

        for step in &self.steps {
            let mut task = Task::new(&step.name, &step.description, &step.role);
            task.agent_id = step.agent_id.clone();

            // Resolve dependencies by name → TaskId.
            for dep_name in &step.depends_on {
                if let Some(dep_id) = name_to_id.get(dep_name) {
                    task.dependencies.push(*dep_id);
                }
            }

            name_to_id.insert(step.name.clone(), task.id);
            tasks.push(task);
        }

        tasks
    }

    /// Built-in workflow template: design → review → develop → test.
    pub fn design_review_develop() -> Self {
        Workflow {
            name: "design-review-develop".into(),
            description: "Standard workflow: architecture design → expert review → implementation → testing".into(),
            steps: vec![
                WorkflowStep {
                    name: "design".into(),
                    role: "architect".into(),
                    description: "Analyze the project requirements and design the architecture. Produce an architecture document with key decisions, component structure, and technology choices.".into(),
                    agent_id: None,
                    depends_on: vec![],
                },
                WorkflowStep {
                    name: "review".into(),
                    role: "reviewer".into(),
                    description: "Review the architecture design produced in the previous step. Identify potential issues, suggest improvements, and validate technical decisions.".into(),
                    agent_id: None,
                    depends_on: vec!["design".into()],
                },
                WorkflowStep {
                    name: "develop".into(),
                    role: "developer".into(),
                    description: "Implement the code based on the reviewed architecture design. Follow the conventions and structure outlined in the architecture document.".into(),
                    agent_id: None,
                    depends_on: vec!["review".into()],
                },
                WorkflowStep {
                    name: "test".into(),
                    role: "tester".into(),
                    description: "Write and execute tests for the implemented code. Ensure all components work correctly and meet the requirements from the architecture document.".into(),
                    agent_id: None,
                    depends_on: vec!["develop".into()],
                },
            ],
        }
    }
}
