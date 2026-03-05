use crate::task::Task;
use std::path::Path;
use tokio::process::Command;

/// The result of validating a task against its requirements.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether all validation requirements passed.
    pub success: bool,
    /// Detailed output or reasons for failure.
    pub details: String,
}

pub struct ValidationEngine;

impl ValidationEngine {
    /// Validates a task by executing all its ValidationRequirements.
    pub async fn validate(task: &Task, workspace: &Path) -> anyhow::Result<ValidationResult> {
        if task.validation_requirements.is_empty() {
            return Ok(ValidationResult {
                success: true,
                details: "No validation requirements specified.".into(),
            });
        }

        let mut all_passed = true;
        let mut details = String::new();

        for (i, req) in task.validation_requirements.iter().enumerate() {
            details.push_str(&format!("--- Validation Step {} ---\n", i + 1));
            details.push_str(&format!("Command: {}\n", req.command));

            // Execute the command in the workspace.
            let output = match Command::new("sh")
                .arg("-c")
                .arg(&req.command)
                .current_dir(workspace)
                .output()
                .await
            {
                Ok(out) => out,
                Err(e) => {
                    all_passed = false;
                    details.push_str(&format!("❌ Failed to execute validation command: {}\n", e));
                    continue;
                }
            };

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined_output = format!("{}\n{}", stdout, stderr);

            details.push_str("Output:\n");
            details.push_str(&combined_output);
            details.push('\n');

            let mut req_passed = output.status.success();

            if !req_passed {
                 details.push_str("❌ Command exited with non-zero status.\n");
            } else {
                 details.push_str("✅ Command exited with success.\n");
            }

            // Check must_include
            for include_term in &req.must_include {
                if !combined_output.contains(include_term) {
                    req_passed = false;
                    details.push_str(&format!("❌ Missing required string: '{}'\n", include_term));
                }
            }

            // Check must_not_include
            for exclude_term in &req.must_not_include {
                if combined_output.contains(exclude_term) {
                    req_passed = false;
                    details.push_str(&format!("❌ Contains forbidden string: '{}'\n", exclude_term));
                }
            }

            if req_passed {
                details.push_str("=> Step Passed.\n\n");
            } else {
                details.push_str("=> STAGE FAILED.\n\n");
                all_passed = false;
            }
        }

        Ok(ValidationResult {
            success: all_passed,
            details,
        })
    }
}
