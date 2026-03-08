//! overclock-ai CLI — Multi-AI Agent Orchestration Tool
//!
//! Entry point for the `overclock-ai` command-line interface.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod commands;

#[derive(Parser)]
#[command(
    name = "overclock-ai",
    about = "🚀 Multi-AI Agent CLI Orchestration Platform",
    long_about = "Overclock-AI coordinates multiple AI CLI tools (CodeBuddy, Kiro, Trae, etc.) \
                  to work on development tasks collaboratively. Each agent works in isolation, \
                  receiving tasks and context from the orchestrator.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new overclock-ai project in the current directory.
    Init {
        /// Project name (defaults to directory name).
        #[arg(long)]
        name: Option<String>,
    },

    /// Manage AI agents and roles configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Create and manage tasks.
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Run a task or workflow.
    Run {
        /// Task ID or workflow name to run.
        target: String,

        /// Run as a workflow (not a single task).
        #[arg(long)]
        workflow: bool,
    },

    /// Check the health status of all configured AI CLI agents.
    Status,

    /// Start the TUI monitor for real-time task tracking.
    Monitor,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// List all configured AI agents.
    Agents,
    /// List all configured roles.
    Roles,
    /// Show current configuration.
    Show,
}

#[derive(Subcommand)]
enum TaskAction {
    /// Create a new task.
    Create {
        /// Task title/description.
        description: String,

        /// Role to assign (architect, reviewer, developer, tester).
        #[arg(long)]
        role: String,

        /// Specific agent to use (overrides role default).
        #[arg(long)]
        agent: Option<String>,
    },

    /// List all tasks.
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => commands::init::run(name).await,
        Commands::Config { action } => match action {
            ConfigAction::Agents => commands::config::agents().await,
            ConfigAction::Roles => commands::config::roles().await,
            ConfigAction::Show => commands::config::show().await,
        },
        Commands::Task { action } => match action {
            TaskAction::Create {
                description,
                role,
                agent,
            } => commands::task::create(description, role, agent).await,
            TaskAction::List => commands::task::list().await,
        },
        Commands::Run { target, workflow } => commands::run::run(target, workflow).await,
        Commands::Status => commands::status::run().await,
        Commands::Monitor => commands::monitor::run_monitor().await,
    }
}
