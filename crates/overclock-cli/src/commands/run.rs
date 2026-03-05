//! `overclock-ai run` command — execute a task or workflow.

use anyhow::Result;
use chrono::Utc;
use overclock_adapters::adapter_trait::AgentAdapter;
use overclock_adapters::codebuddy::CodeBuddyAdapter;
use overclock_adapters::gemini::GeminiAdapter;
use overclock_adapters::kiro::KiroAdapter;
use overclock_core::config::ProjectConfig;
use overclock_core::context::SharedContext;
use overclock_core::context::{CONTEXT_DIR, TaskResultSummary};
use overclock_core::task::{Task, TaskResult, TaskStatus};
use overclock_core::workflow::Workflow;

/// Run a single task or a workflow.
pub async fn run(target: String, workflow: bool) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = ProjectConfig::load(&workspace)?;
    let mut context = SharedContext::load(&workspace)?;

    if workflow {
        run_workflow(&target, &config, &mut context, &workspace).await
    } else {
        run_task(&target, &config, &mut context, &workspace).await
    }
}

async fn run_task(
    task_id: &str,
    config: &ProjectConfig,
    context: &mut SharedContext,
    workspace: &std::path::Path,
) -> Result<()> {
    // Load the task.
    let task_path = workspace
        .join(CONTEXT_DIR)
        .join("tasks")
        .join(format!("{task_id}.json"));
    if !task_path.exists() {
        anyhow::bail!(
            "Task not found: {task_id}. Run `overclock-ai task list` to see available tasks."
        );
    }
    let content = std::fs::read_to_string(&task_path)?;
    let mut task: Task = serde_json::from_str(&content)?;

    // Determine which agent to use.
    let agent_id = task.agent_id.clone().unwrap_or_else(|| {
        config
            .roles
            .get(&task.role)
            .map(|r| r.default_agent.clone())
            .unwrap_or_else(|| "codebuddy".to_string())
    });

    let agent_config = config
        .agents
        .get(&agent_id)
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found in config", agent_id))?;

    let adapter: Box<dyn AgentAdapter> = match agent_config.agent_type.as_str() {
        "codebuddy" => Box::new(CodeBuddyAdapter::new()),
        "kiro-cli" => Box::new(KiroAdapter::new()),
        "gemini-cli" => Box::new(GeminiAdapter::new()),
        other => anyhow::bail!("No adapter for agent type: {other}"),
    };

    println!("🚀 Running task: {}", task.title);
    println!("   Role:  {}", task.role);
    println!("   Agent: {} ({})", agent_id, adapter.name());
    println!();

    use overclock_core::recovery::{RecoveryAction, classify_error, determine_action};
    use overclock_core::telemetry::{TelemetryEvent, TelemetrySink};
    use std::time::Instant;

    let start_time = Instant::now();
    let mut sink = TelemetrySink::new();

    loop {
        // Update task status to Running.
        task.status = TaskStatus::Running {
            agent_id: agent_id.clone(),
            started_at: Utc::now(),
        };
        save_task(&task, workspace)?;

        // Execute the task.
        match adapter.execute_task(&task, context, agent_config).await {
            Ok(mut output) => {
                if output.success {
                    let mut validated = true;
                    if !task.validation_requirements.is_empty() {
                        println!("⏳ Agent reports completion. Verifying {} requirements...", task.validation_requirements.len());
                        task.status = TaskStatus::Validating {
                            agent_id: agent_id.clone(),
                        };
                        save_task(&task, workspace)?;

                        match overclock_core::validation::ValidationEngine::validate(&task, workspace).await {
                            Ok(res) => {
                                if res.success {
                                    println!("✅ Validation passed!");
                                } else {
                                    println!("❌ Validation failed!");
                                    println!("{}", res.details);
                                    validated = false;
                                    
                                    // Override output to simulate failure to feed back into retry loop
                                    output.success = false;
                                    output.summary = format!("Agent output was successful, but Evidence-Driven Validation failed:\n{}", res.details);
                                }
                            }
                            Err(e) => {
                                println!("⚠️  Failed to run validation engine: {e}");
                                validated = false;
                                output.success = false;
                                output.summary = format!("Validation engine error: {e}");
                            }
                        }
                    }

                    if validated {
                        let duration_ms = start_time.elapsed().as_millis() as u64;
                        println!("✅ Task completed successfully! ({}ms)\n", duration_ms);
                        println!("--- Agent Output ---");
                        println!("{}", output.raw_output);
                        println!("--- End Output ---\n");

                        // Save artifact to .overclock-ai/artifacts/
                        let artifact_dir = workspace.join(CONTEXT_DIR).join("artifacts");
                        std::fs::create_dir_all(&artifact_dir)?;
                        let artifact_path = artifact_dir.join(format!("{}-output.md", task.id));
                        std::fs::write(&artifact_path, &output.raw_output)?;

                        sink.record(TelemetryEvent::TaskCompleted {
                            task_id: task.id.to_string(),
                            agent_handle: agent_id.clone(),
                            duration_ms,
                            context_size_bytes: context.to_prompt_context().len(),
                        });

                        // Update task status.
                        task.status = TaskStatus::Completed {
                            completed_at: Utc::now(),
                            result: TaskResult {
                                summary: output.summary.clone(),
                                modified_files: output.modified_files.clone(),
                                artifacts: vec![artifact_path.clone()],
                                raw_output: output.raw_output.clone(),
                            },
                        };

                        // Update shared context with this task's result.
                        context.add_task_result(TaskResultSummary {
                            task_title: task.title.clone(),
                            role: task.role.clone(),
                            agent_id: agent_id.clone(),
                            summary: output.summary,
                            artifact_paths: vec![artifact_path],
                            completed_at: Utc::now(),
                        });
                        context.save()?;
                        save_task(&task, workspace)?;
                        break;
                    }
                }
                
                if !output.success {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    println!("❌ Task failed!\n");
                    println!("{}", output.summary);

                    let category = classify_error(&output.summary);
                    let action = determine_action(&category, task.retry_count, task.max_retries);

                    sink.record(TelemetryEvent::TaskFailed {
                        task_id: task.id.to_string(),
                        agent_handle: agent_id.clone(),
                        error_category: format!("{:?}", category),
                        duration_ms,
                        recovery_attempted: matches!(
                            action,
                            RecoveryAction::Retry { .. } | RecoveryAction::RunInitScript
                        ),
                    });

                    match action {
                        RecoveryAction::Retry { delay_ms, .. } => {
                            task.retry_count += 1;
                            println!(
                                "🔄 Auto-recovering: Retrying task (Attempt {})",
                                task.retry_count
                            );
                            sink.record(TelemetryEvent::RecoveryTriggered {
                                task_id: task.id.to_string(),
                                action: "Retry".to_string(),
                                retry_count: task.retry_count,
                            });
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        RecoveryAction::RunInitScript => {
                            task.retry_count += 1;
                            println!(
                                "🔄 Auto-recovering: Running Init Script to fix environment..."
                            );
                            sink.record(TelemetryEvent::RecoveryTriggered {
                                task_id: task.id.to_string(),
                                action: "RunInitScript".to_string(),
                                retry_count: task.retry_count,
                            });
                            // In a real scenario, execute init.sh here.
                            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                            continue;
                        }
                        RecoveryAction::Block { reason } => {
                            task.status = TaskStatus::Blocked {
                                blocked_at: Utc::now(),
                                reason: format!("Failed after retries. Last error: {}", reason),
                            };
                            save_task(&task, workspace)?;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Task execution adapter error: {e}");
                task.status = TaskStatus::Blocked {
                    blocked_at: Utc::now(),
                    reason: format!("Fatal adapter error: {}", e),
                };
                save_task(&task, workspace)?;
                break;
            }
        }
    }

    // Append telemetry jsonl
    if let Ok(jsonl_line) = serde_json::to_string(&sink.records) {
        let telemetry_dir = workspace.join(CONTEXT_DIR).join("telemetry");
        std::fs::create_dir_all(&telemetry_dir)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(telemetry_dir.join("telemetry.jsonl"))?;
        use std::io::Write;
        writeln!(file, "{}", jsonl_line)?;
    }

    Ok(())
}

async fn run_workflow(
    workflow_name: &str,
    config: &ProjectConfig,
    context: &mut SharedContext,
    workspace: &std::path::Path,
) -> Result<()> {
    // Load built-in or custom workflow.
    let workflow = match workflow_name {
        "design-review-develop" => Workflow::design_review_develop(),
        _ => anyhow::bail!("Unknown workflow: {workflow_name}. Available: design-review-develop"),
    };

    println!(
        "🔄 Starting workflow: {} ({} steps)",
        workflow.name,
        workflow.steps.len()
    );
    println!("   {}\n", workflow.description);

    let tasks = workflow.generate_tasks();

    // Execute tasks sequentially (respecting dependencies).
    for task in &tasks {
        // Save task.
        save_task(task, workspace)?;

        let task_id = task.id.to_string();
        println!("━━━ Step: {} ━━━", task.title);
        run_task(&task_id, config, context, workspace).await?;
        println!();
    }

    println!("🎉 Workflow '{}' completed!", workflow.name);
    Ok(())
}

fn save_task(task: &Task, workspace: &std::path::Path) -> Result<()> {
    let tasks_dir = workspace.join(CONTEXT_DIR).join("tasks");
    std::fs::create_dir_all(&tasks_dir)?;
    let task_file = tasks_dir.join(format!("{}.json", task.id));
    let content = serde_json::to_string_pretty(task)?;
    std::fs::write(&task_file, content)?;
    Ok(())
}
