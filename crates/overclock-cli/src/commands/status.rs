//! `overclock-ai status` command — check health of all configured agents.

use anyhow::Result;
use overclock_core::config::ProjectConfig;
use overclock_adapters::adapter_trait::AgentAdapter;
use overclock_adapters::codebuddy::CodeBuddyAdapter;
use overclock_adapters::kiro::KiroAdapter;
use overclock_adapters::trae::TraeAdapter;

/// Check health status of all configured agents.
pub async fn run() -> Result<()> {
    let workspace = std::env::current_dir()?;
    let config = ProjectConfig::load(&workspace)?;

    println!("🔍 Checking AI CLI agent status...\n");
    println!("{:<15} {:<15} {:<15} {:<40}", "Agent ID", "Type", "Status", "Details");
    println!("{}", "-".repeat(85));

    for (id, agent_config) in &config.agents {
        let adapter: Box<dyn AgentAdapter> = match agent_config.agent_type.as_str() {
            "codebuddy" => Box::new(CodeBuddyAdapter::new()),
            "kiro-cli" => Box::new(KiroAdapter::new()),
            "trae-agent" => Box::new(TraeAdapter::new()),
            _ => {
                println!(
                    "{:<15} {:<15} {:<15} {:<40}",
                    id, agent_config.agent_type, "⚠️  Unknown", "No adapter for this agent type"
                );
                continue;
            }
        };

        let health = adapter.health_check().await;
        let (status, details) = match &health {
            overclock_adapters::adapter_trait::HealthStatus::Ready { version } => {
                ("✅ Ready", version.clone())
            }
            overclock_adapters::adapter_trait::HealthStatus::NotInstalled { reason } => {
                ("❌ Missing", reason.clone())
            }
            overclock_adapters::adapter_trait::HealthStatus::NotAuthenticated { reason } => {
                ("🔐 No Auth", reason.clone())
            }
            overclock_adapters::adapter_trait::HealthStatus::Error { reason } => {
                ("⚠️  Error", reason.clone())
            }
        };

        // Truncate details for display.
        let details_display: String = details.chars().take(38).collect();
        println!(
            "{:<15} {:<15} {:<15} {:<40}",
            id, agent_config.agent_type, status, details_display
        );
    }

    Ok(())
}
