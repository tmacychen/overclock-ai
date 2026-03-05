//! `overclock-ai task` commands — create and list tasks.

use anyhow::Result;
use overclock_core::context::CONTEXT_DIR;
use overclock_core::task::Task;

/// Create a new task.
pub async fn create(description: String, role: String, agent: Option<String>) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let mut task = Task::new(&description, &description, &role);
    task.agent_id = agent.clone();

    // Save the task to .overclock-ai/tasks/
    let tasks_dir = workspace.join(CONTEXT_DIR).join("tasks");
    std::fs::create_dir_all(&tasks_dir)?;
    let task_file = tasks_dir.join(format!("{}.json", task.id));
    let content = serde_json::to_string_pretty(&task)?;
    std::fs::write(&task_file, content)?;

    println!("✅ Task created:");
    println!("   ID:    {}", task.id);
    println!("   Title: {}", task.title);
    println!("   Role:  {}", task.role);
    if let Some(agent_id) = &task.agent_id {
        println!("   Agent: {agent_id}");
    } else {
        println!("   Agent: (role default)");
    }
    println!("\n   Run with: overclock-ai run {}", task.id);

    Ok(())
}

/// List all tasks.
pub async fn list() -> Result<()> {
    let workspace = std::env::current_dir()?;
    let tasks_dir = workspace.join(CONTEXT_DIR).join("tasks");

    if !tasks_dir.exists() {
        println!("📋 No tasks found. Run `overclock-ai init` first.");
        return Ok(());
    }

    let mut tasks = Vec::new();
    for entry in std::fs::read_dir(&tasks_dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == "json") {
            let content = std::fs::read_to_string(entry.path())?;
            if let Ok(task) = serde_json::from_str::<Task>(&content) {
                tasks.push(task);
            }
        }
    }

    if tasks.is_empty() {
        println!("📋 No tasks found. Create one with `overclock-ai task create`.");
        return Ok(());
    }

    // Sort by creation time.
    tasks.sort_by_key(|t| t.created_at);

    println!("📋 Tasks ({} total):\n", tasks.len());
    println!(
        "{:<38} {:<30} {:<12} {:<10}",
        "ID", "Title", "Role", "Status"
    );
    println!("{}", "-".repeat(90));
    for task in &tasks {
        let status = match &task.status {
            overclock_core::task::TaskStatus::Pending => "⏳ Pending",
            overclock_core::task::TaskStatus::Assigned { .. } => "📌 Assigned",
            overclock_core::task::TaskStatus::Running { .. } => "🔄 Running",
            overclock_core::task::TaskStatus::Validating { .. } => "⏳ Validating",
            overclock_core::task::TaskStatus::AwaitingReview { .. } => "👀 Review",
            overclock_core::task::TaskStatus::Completed { .. } => "✅ Done",
            overclock_core::task::TaskStatus::Blocked { .. } => "❌ Blocked",
        };
        let title: String = task.title.chars().take(28).collect();
        println!(
            "{:<38} {:<30} {:<12} {:<10}",
            task.id, title, task.role, status
        );
    }

    Ok(())
}
