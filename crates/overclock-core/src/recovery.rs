//! Error Recovery System for Agent Actions
//!
//! Based on ADDS v2.1 specifications, this module categorizes errors and
//! dictates whether an automatic recovery attempt (e.g. retry, run init.sh)
//! should be executed or if the task should be marked as blocked.

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Categories of errors an Agent CLI might produce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Environment issues like missing directories, python venv not found.
    Environment,
    /// Missing dependencies, node_modules not found, import errors.
    Dependency,
    /// Code syntax error, type error, compilation failed.
    CodeLogic,
    /// The prompt was too ambiguous or the agent explicitly asked for clarification.
    AmbiguousRequirement,
    /// Network failed, LLM API rate limit, API key invalid.
    Infrastructure,
    /// General or unknown error.
    Unknown,
}

/// A decision made by the recovery system on how to handle an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Action is automatically retryable (e.g. after fixing something, or just transient).
    Retry { max_retries: u32, delay_ms: u64 },
    /// Run an initialization script before retrying.
    RunInitScript,
    /// Abort execution. Wait for human intervention.
    Block { reason: String },
}

/// Analyzes an error string and categorizes it into a known pattern.
pub fn classify_error(error_output: &str) -> ErrorCategory {
    let output = error_output.to_lowercase();

    if output.contains("no such file or directory") || output.contains("env: ") {
        ErrorCategory::Environment
    } else if output.contains("module not found")
        || output.contains("importerror")
        || output.contains("cannot find module")
    {
        ErrorCategory::Dependency
    } else if output.contains("syntaxerror")
        || output.contains("typeerror")
        || output.contains("compile error")
        || output.contains("build failed")
    {
        ErrorCategory::CodeLogic
    } else if output.contains("rate limit")
        || output.contains("connection refused")
        || output.contains("timeout")
        || output.contains("api key")
    {
        ErrorCategory::Infrastructure
    } else if output.contains("clarif")
        || output.contains("ambiguous")
        || output.contains("need more info")
    {
        ErrorCategory::AmbiguousRequirement
    } else {
        ErrorCategory::Unknown
    }
}

/// Determines the recovery action based on the error category and current retry count.
pub fn determine_action(
    category: &ErrorCategory,
    current_retry: u32,
    max_retries: u32,
) -> RecoveryAction {
    if current_retry >= max_retries {
        return RecoveryAction::Block {
            reason: format!("Max retries ({}) exceeded for {:?}", max_retries, category),
        };
    }

    match category {
        ErrorCategory::Environment | ErrorCategory::Dependency => {
            // Environment/deps might be solvable by running the project's init script
            RecoveryAction::RunInitScript
        }
        ErrorCategory::CodeLogic => {
            // Auto-retry to let the agent fix its own syntax error
            let delay_ms = calculate_exponential_backoff(current_retry, 1000, 10000);
            RecoveryAction::Retry { 
                max_retries, 
                delay_ms,
            }
        }
        ErrorCategory::Infrastructure => {
            // Transient network issues. Exponential backoff.
            let delay_ms = calculate_exponential_backoff(current_retry, 2000, 30000);
            RecoveryAction::Retry { 
                max_retries, 
                delay_ms,
            }
        }
        ErrorCategory::AmbiguousRequirement => {
            // Unlikely to auto-resolve without context change
            RecoveryAction::Block { 
                reason: "Agent flagged ambiguous requirements".into(),
            }
        }
        ErrorCategory::Unknown => {
            // Try resetting or just blocking. For now, block to be safe.
            RecoveryAction::Block { 
                reason: "Unknown error category, safer to block".into(),
            }
        }
    }
}

/// Calculate exponential backoff delay with jitter.
/// 
/// # Arguments
/// * `attempt` - Current retry attempt (0-based)
/// * `base_delay` - Base delay in milliseconds
/// * `max_delay` - Maximum delay in milliseconds
/// 
/// # Returns
/// Calculated delay in milliseconds
pub fn calculate_exponential_backoff(attempt: u32, base_delay: u64, max_delay: u64) -> u64 {
    // Calculate exponential backoff: base_delay * 2^attempt
    let delay = base_delay.saturating_mul(2_u64.saturating_pow(attempt));
    
    // Add some jitter (±20%) to avoid thundering herd
    let mut rng = rand::thread_rng();
    let jitter = (delay as f64 * 0.2 * (rng.gen_range(-0.5..0.5))) as i64;
    let delay_with_jitter = delay.saturating_add(jitter.abs() as u64);
    
    // Cap at max_delay
    std::cmp::min(delay_with_jitter, max_delay)
}
