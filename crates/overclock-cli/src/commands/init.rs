//! `overclock-ai init` command — initialize a new project.

use anyhow::Result;
use overclock_core::config::ProjectConfig;
use overclock_core::context::{SharedContext, CONTEXT_DIR};
use std::path::PathBuf;
use tracing::info;

pub async fn run(name: Option<String>) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let project_name = name.unwrap_or_else(|| {
        workspace
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string()
    });

    println!("🚀 Initializing overclock-ai project: {project_name}");

    // Check if already initialized.
    let config_path = workspace.join(overclock_core::config::CONFIG_FILE);
    if config_path.exists() {
        println!("⚠️  Project already initialized (overclock-ai.toml exists).");
        return Ok(());
    }

    // Generate default config.
    let config = ProjectConfig::default_config(&project_name);
    config.save(&workspace)?;
    println!("📄 Created overclock-ai.toml");

    // Create the .overclock-ai/ directory structure.
    let ctx_dir = workspace.join(CONTEXT_DIR);
    let dirs_to_create = [
        ctx_dir.clone(),
        ctx_dir.join("tasks"),
        ctx_dir.join("artifacts"),
        ctx_dir.join("history"),
    ];
    for dir in &dirs_to_create {
        std::fs::create_dir_all(dir)?;
    }
    println!("📁 Created .overclock-ai/ directory structure");

    // Initialize empty shared context.
    let mut context = SharedContext::new(workspace.clone());
    context.project_brief = format!("Project: {project_name}");
    context.save()?;
    println!("📋 Initialized shared context");

    // Create template files.
    let templates_dir = workspace.join("templates");
    std::fs::create_dir_all(&templates_dir)?;
    create_template(&templates_dir, "architect.md", ARCHITECT_TEMPLATE)?;
    create_template(&templates_dir, "reviewer.md", REVIEWER_TEMPLATE)?;
    create_template(&templates_dir, "developer.md", DEVELOPER_TEMPLATE)?;
    create_template(&templates_dir, "tester.md", TESTER_TEMPLATE)?;
    println!("📝 Created prompt templates");

    // Add .overclock-ai to .gitignore if not already present.
    let gitignore = workspace.join(".gitignore");
    let gitignore_entry = format!("\n# Overclock-AI\n{CONTEXT_DIR}/\n");
    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore)?;
        if !content.contains(CONTEXT_DIR) {
            std::fs::write(&gitignore, format!("{content}{gitignore_entry}"))?;
            println!("📎 Updated .gitignore");
        }
    } else {
        std::fs::write(&gitignore, gitignore_entry)?;
        println!("📎 Created .gitignore");
    }

    println!("\n✅ Project initialized! Next steps:");
    println!("   1. Edit overclock-ai.toml to configure your AI agents");
    println!("   2. Run `overclock-ai status` to check agent availability");
    println!("   3. Run `overclock-ai task create \"...\" --role architect` to create tasks");

    info!("Project initialized at {}", workspace.display());
    Ok(())
}

fn create_template(dir: &PathBuf, filename: &str, content: &str) -> Result<()> {
    let path = dir.join(filename);
    if !path.exists() {
        std::fs::write(&path, content)?;
    }
    Ok(())
}

const ARCHITECT_TEMPLATE: &str = r#"# Architect Role

You are a software architect. Your task is to:

1. Analyze the project requirements
2. Design the system architecture
3. Make technology decisions with clear rationale
4. Define component structure and interfaces
5. Document architecture decisions

## Output Format

Provide:
- Architecture overview diagram (if applicable)
- Component breakdown
- Technology stack with rationale
- Key interfaces and data flow
- Architecture Decision Records (ADRs)
"#;

const REVIEWER_TEMPLATE: &str = r#"# Reviewer Role

You are a senior code/design reviewer. Your task is to:

1. Review the provided design, code, or documentation
2. Identify potential issues, risks, and improvements
3. Validate technical decisions
4. Suggest concrete improvements

## Output Format

Provide:
- Summary of reviewed items
- Issues found (categorized: critical, major, minor)
- Improvement suggestions with rationale
- Approval status: APPROVED / CHANGES_REQUESTED
"#;

const DEVELOPER_TEMPLATE: &str = r#"# Developer Role

You are a software developer. Your task is to:

1. Implement code based on the provided design/requirements
2. Follow coding conventions and project structure
3. Write clean, well-documented code
4. Handle error cases appropriately

## Output Format

Provide:
- Summary of implemented changes
- List of files created/modified
- Notes on design decisions made during implementation
"#;

const TESTER_TEMPLATE: &str = r#"# Tester Role

You are a software tester. Your task is to:

1. Write tests for the provided code
2. Execute tests and report results
3. Identify edge cases and boundary conditions
4. Verify requirements are met

## Output Format

Provide:
- Test plan summary
- Tests written (unit, integration, e2e)
- Test execution results
- Coverage report (if applicable)
- Issues found during testing
"#;
