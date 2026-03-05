//! `overclock-ai run` command — execute a task or workflow.

use anyhow::Result;
use overclock_core::config::ProjectConfig;
use overclock_core::context::SharedContext;
use overclock_core::task::{Task, TaskStatus, TaskResult};
use overclock_core::context::{CONTEXT_DIR, TaskResultSummary};
use overclock_core::workflow::Workflow;
use overclock_adapters::adapter_trait::AgentAdapter;
use overclock_adapters::codebuddy::CodeBuddyAdapter;
use overclock_adapters::kiro::KiroAdapter;
use overclock_adapters::trae::TraeAdapter;
use chrono::Utc;

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
    let task_path = workspace.join(CONTEXT_DIR).join("tasks").join(format!("{task_id}.json"));
    if !task_path.exists() {
        anyhow::bail!("Task not found: {task_id}. Run `overclock-ai task list` to see available tasks.");
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
        "trae-agent" => Box::new(TraeAdapter::new()),
        other => anyhow::bail!("No adapter for agent type: {other}"),
    };

    println!("🚀 Running task: {}", task.title);
    println!("   Role:  {}", task.role);
    println!("   Agent: {} ({})", agent_id, adapter.name());
    println!();

    // Update task status to Running.
    task.status = TaskStatus::Running {
        agent_id: agent_id.clone(),
        started_at: Utc::now(),
    };
    save_task(&task, workspace)?;

    // Execute the task.
    match adapter.execute_task(&task, context, agent_config).await {
        Ok(output) => {
            if output.success {
                println!("✅ Task completed successfully!\n");
                println!("--- Agent Output ---");
                println!("{}", output.raw_output);
                println!("--- End Output ---\n");

                // Update task status.
                task.status = TaskStatus::Completed {
                    completed_at: Utc::now(),
                    result: TaskResult {
                        summary: output.summary.clone(),
                        modified_files: output.modified_files.clone(),
                        artifacts: output.artifacts.clone(),
                        raw_output: output.raw_output.clone(),
                    },
                };

                // Save artifact to .overclock-ai/artifacts/
                let artifact_dir = workspace.join(CONTEXT_DIR).join("artifacts");
                std::fs::create_dir_all(&artifact_dir)?;
                let artifact_path = artifact_dir.join(format!("{}-output.md", task.id));
                std::fs::write(&artifact_path, &output.raw_output)?;

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
            } else {
                println!("❌ Task failed!\n");
                println!("{}", output.summary);
                task.status = TaskStatus::Failed {
                    failed_at: Utc::now(),
                    error: output.summary,
                };
            }
            save_task(&task, workspace)?;
        }
        Err(e) => {
            println!("❌ Task execution error: {e}");
            task.status = TaskStatus::Failed {
                failed_at: Utc::now(),
                error: e.to_string(),
            };
            save_task(&task, workspace)?;
        }
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

    println!("🔄 Starting workflow: {} ({} steps)", workflow.name, workflow.steps.len());
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
