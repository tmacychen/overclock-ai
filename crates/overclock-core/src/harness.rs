//! Agent Harness for reliable AI agent execution.
//! 
//! The Agent Harness provides a robust execution environment for AI agents, including:
//! 1. Evidence-driven validation: Agents must provide structured evidence of task completion
//! 2. Automated error recovery: Classification of errors and automatic retry/reset logic
//! 3. Closed-loop data collection: Capture execution metrics and failure reasons

use crate::context::SharedContext;
use crate::event::{EventBus, OrchestratorEvent};
use crate::recovery::{classify_error, determine_action, RecoveryAction};
use crate::task::{Task, TaskResult, TaskStatus};
use crate::telemetry::{TelemetryEvent, TelemetrySink};
use crate::validation::ValidationEngine;
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Configuration for the Agent Harness
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    /// Maximum number of automatic retries per task
    pub max_retries: u32,
    /// Timeout for agent execution in seconds
    pub execution_timeout: u64,
    /// Maximum context size in bytes
    pub max_context_size: usize,
    /// Whether to collect telemetry data
    pub collect_telemetry: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            execution_timeout: 300, // 5 minutes
            max_context_size: 100_000, // 100KB
            collect_telemetry: true,
        }
    }
}

/// The Agent Harness
pub struct AgentHarness {
    config: HarnessConfig,
    event_bus: EventBus,
    telemetry_sink: TelemetrySink,
}

impl AgentHarness {
    /// Create a new Agent Harness
    pub fn new(config: HarnessConfig, event_bus: EventBus, telemetry_sink: TelemetrySink) -> Self {
        Self {
            config,
            event_bus,
            telemetry_sink,
        }
    }

    /// Execute a task with the harness
    pub async fn execute_task(
        &mut self,
        task: &mut Task,
        context: &SharedContext,
        mut agent_execute: impl FnMut(&Task, &str) -> tokio::task::JoinHandle<anyhow::Result<TaskResult>>,
    ) -> anyhow::Result<TaskResult> {
        let start_time = Utc::now();
        let _task_id = task.id;
        let agent_id = task.agent_id.clone().unwrap_or_else(|| "default".to_string());

        // Build context with compression
        let prompt_context = context.to_prompt_context_with_limit(Some(self.config.max_context_size));

        // Event: Task assigned
        self.event_bus.emit(OrchestratorEvent::TaskAssigned {
            task_id: task.id,
            agent_id: agent_id.clone(),
            timestamp: Utc::now(),
        });

        // Event: Agent started
        self.event_bus.emit(OrchestratorEvent::AgentStarted {
            task_id: task.id,
            agent_id: agent_id.clone(),
            timestamp: Utc::now(),
        });

        let mut attempt = 0;
        loop {
            attempt += 1;
            task.retry_count = attempt - 1;

            if attempt > self.config.max_retries {
                let error = format!("Max retries ({}) exceeded", self.config.max_retries);
                task.status = TaskStatus::Blocked {
                    blocked_at: Utc::now(),
                    reason: error.clone(),
                };

                // Telemetry: Task failed
                if self.config.collect_telemetry {
                    self.telemetry_sink.record(TelemetryEvent::TaskFailed {
                        task_id: task.id.to_string(),
                        agent_handle: agent_id.clone(),
                        error_category: "MaxRetriesExceeded".to_string(),
                        duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
                        recovery_attempted: true,
                    });
                }

                return Err(anyhow::anyhow!(error));
            }

            // Execute the task with timeout
            let execution = tokio::time::timeout(
                Duration::from_secs(self.config.execution_timeout),
                agent_execute(task, &prompt_context),
            );

            match execution.await {
                Ok(join_result) => {
                    match join_result {
                        Ok(task_result) => {
                            // Task completed successfully, validate the result
                            let task_result = task_result?;
                            let validation_result = ValidationEngine::validate(task, &context.workspace_root).await?;

                            if validation_result.success {
                                // Task completed and validated
                                let completed_at = Utc::now();
                                task.status = TaskStatus::Completed {
                                    completed_at,
                                    result: task_result,
                                };

                                // Event: Task completed
                                self.event_bus.emit(OrchestratorEvent::TaskCompleted {
                                    task_id: task.id,
                                    agent_id: agent_id.clone(),
                                    summary: "Task completed successfully".to_string(),
                                    timestamp: completed_at,
                                });

                                // Telemetry: Task completed
                                if self.config.collect_telemetry {
                                    self.telemetry_sink.record(TelemetryEvent::TaskCompleted {
                                        task_id: task.id.to_string(),
                                        agent_handle: agent_id.clone(),
                                        duration_ms: (completed_at - start_time).num_milliseconds() as u64,
                                        context_size_bytes: prompt_context.len(),
                                    });
                                }

                                // Return the result by cloning it from the task status
                                if let TaskStatus::Completed { result: ref completed_result, .. } = task.status {
                                    return Ok(completed_result.clone());
                                } else {
                                    return Err(anyhow::anyhow!("Task status was not set to Completed"));
                                }
                            } else {
                                // Validation failed, treat as error
                                let error = format!("Validation failed: {}", validation_result.details);
                                self.handle_error(task, &agent_id, &error, &start_time).await?;
                            }
                        }
                        Err(e) => {
                            // Execution failed
                            let error = e.to_string();
                            self.handle_error(task, &agent_id, &error, &start_time).await?;
                        }
                    }
                }
                Err(_) => {
                    // Execution timed out
                    let error = format!("Execution timed out after {} seconds", self.config.execution_timeout);
                    self.handle_error(task, &agent_id, &error, &start_time).await?;
                }
            }
        }
    }

    /// Handle an error during task execution
    async fn handle_error(
        &mut self,
        task: &mut Task,
        agent_id: &str,
        error: &str,
        start_time: &DateTime<Utc>,
    ) -> anyhow::Result<()> {
        // Classify the error
        let error_category = classify_error(error);

        // Determine recovery action
        let action = determine_action(&error_category, task.retry_count, self.config.max_retries);

        // Telemetry: Recovery triggered
        if self.config.collect_telemetry {
            self.telemetry_sink.record(TelemetryEvent::RecoveryTriggered {
                task_id: task.id.to_string(),
                action: format!("{:?}", action),
                retry_count: task.retry_count,
            });
        }

        match action {
            RecoveryAction::Retry { max_retries, delay_ms } => {
                // Wait before retrying
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                
                // Event: Agent output (error)
                self.event_bus.emit(OrchestratorEvent::AgentOutput {
                    task_id: task.id,
                    agent_id: agent_id.to_string(),
                    content: format!("Error: {}. Retrying... (Attempt {}/{})\n", error, task.retry_count + 1, max_retries),
                    timestamp: Utc::now(),
                });
            }
            RecoveryAction::RunInitScript => {
                // Run init script
                self.event_bus.emit(OrchestratorEvent::AgentOutput {
                    task_id: task.id,
                    agent_id: agent_id.to_string(),
                    content: format!("Error: {}. Running init script...\n", error),
                    timestamp: Utc::now(),
                });

                // TODO: Implement init script execution
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            RecoveryAction::Block { reason } => {
                // Block the task
                task.status = TaskStatus::Blocked {
                    blocked_at: Utc::now(),
                    reason: format!("{}: {}", reason, error),
                };

                // Event: Task failed
                self.event_bus.emit(OrchestratorEvent::TaskFailed {
                    task_id: task.id,
                    agent_id: agent_id.to_string(),
                    error: error.to_string(),
                    timestamp: Utc::now(),
                });

                // Telemetry: Task failed
                if self.config.collect_telemetry {
                    self.telemetry_sink.record(TelemetryEvent::TaskFailed {
                        task_id: task.id.to_string(),
                        agent_handle: agent_id.to_string(),
                        error_category: format!("{:?}", error_category),
                        duration_ms: (Utc::now() - *start_time).num_milliseconds() as u64,
                        recovery_attempted: true,
                    });
                }

                return Err(anyhow::anyhow!(reason));
            }
        }

        Ok(())
    }

    /// Flush telemetry data
    pub fn flush_telemetry(&mut self) -> anyhow::Result<()> {
        self.telemetry_sink.flush()
    }
}
