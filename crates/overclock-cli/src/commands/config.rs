//! `overclock-ai config` commands — show agents, roles, and config.

use anyhow::Result;
use overclock_core::config::ProjectConfig;

fn load_config() -> Result<ProjectConfig> {
    let workspace = std::env::current_dir()?;
    ProjectConfig::load(&workspace)
}

/// List all configured AI agents.
pub async fn agents() -> Result<()> {
    let config = load_config()?;
    println!("🤖 Configured AI Agents:\n");
    println!("{:<15} {:<15} {:<20} {:<10}", "ID", "Type", "Binary", "Free Tier");
    println!("{}", "-".repeat(60));
    for (id, agent) in &config.agents {
        println!(
            "{:<15} {:<15} {:<20} {:<10}",
            id,
            agent.agent_type,
            agent.binary.as_deref().unwrap_or("-"),
            if agent.free_tier { "✅ Yes" } else { "❌ No" }
        );
    }
    Ok(())
}

/// List all configured roles.
pub async fn roles() -> Result<()> {
    let config = load_config()?;
    println!("🎭 Configured Roles:\n");
    println!("{:<15} {:<20} {:<40}", "Role", "Default Agent", "Description");
    println!("{}", "-".repeat(75));
    for (name, role) in &config.roles {
        println!(
            "{:<15} {:<20} {:<40}",
            name, role.default_agent, role.description
        );
    }
    Ok(())
}

/// Show the entire configuration.
pub async fn show() -> Result<()> {
    let config = load_config()?;
    let output = toml::to_string_pretty(&config)?;
    println!("📄 Current Configuration (overclock-ai.toml):\n");
    println!("{output}");
    Ok(())
}
