//! Telemetry and Data Collection
//!
//! Based on ADDS v2.1 specifications, this module is responsible for capturing
//! success, failure, and performance data points, saving them locally for future
//! model fine-tuning and process evaluation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Core telemetry event representing any measurable occurrence in the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum TelemetryEvent {
    /// A task was successfully completed and validated.
    TaskCompleted {
        task_id: String,
        agent_handle: String,
        duration_ms: u64,
        context_size_bytes: usize,
    },
    /// A task failed or was blocked.
    TaskFailed {
        task_id: String,
        agent_handle: String,
        error_category: String,
        duration_ms: u64,
        recovery_attempted: bool,
    },
    /// An automated recovery action was triggered.
    RecoveryTriggered {
        task_id: String,
        action: String,
        retry_count: u32,
    },
    /// General system or orchestrator performance metric.
    SystemMetric { metric_name: String, value_ms: u64 },
}

/// A wrapper around telemetry events tying them to a timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    pub timestamp: DateTime<Utc>,
    pub event: TelemetryEvent,
}

impl TelemetryRecord {
    pub fn new(event: TelemetryEvent) -> Self {
        Self {
            timestamp: Utc::now(),
            event,
        }
    }
}

/// A telemetry sink for collecting and writing telemetry data to disk.
/// Writes records to JSONL files in the `.ai/training_data/` directory.
pub struct TelemetrySink {
    // In memory store for records
    pub records: Vec<TelemetryRecord>,
    // Base directory for telemetry data
    base_dir: PathBuf,
}

impl TelemetrySink {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            records: Vec::new(),
            base_dir: base_dir.join(".ai").join("training_data"),
        }
    }

    pub fn record(&mut self, event: TelemetryEvent) {
        self.records.push(TelemetryRecord::new(event));
    }

    /// Convert recorded lines to JSONL (JSON Lines) format typical for training data.
    pub fn to_jsonl(&self) -> String {
        self.records
            .iter()
            .filter_map(|r| serde_json::to_string(r).ok())
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// Flush all records to disk in JSONL format.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        // Create the directory if it doesn't exist
        fs::create_dir_all(&self.base_dir)?;

        // Write records to the appropriate file based on event type
        let mut success_records = Vec::new();
        let mut failure_records = Vec::new();
        let mut performance_records = Vec::new();

        for record in &self.records {
            match &record.event {
                TelemetryEvent::TaskCompleted { .. } => success_records.push(record),
                TelemetryEvent::TaskFailed { .. } => failure_records.push(record),
                TelemetryEvent::RecoveryTriggered { .. } => failure_records.push(record),
                TelemetryEvent::SystemMetric { .. } => performance_records.push(record),
            }
        }

        // Write success records
        if !success_records.is_empty() {
            self.write_records(&success_records, "successes.jsonl")?;
        }

        // Write failure records
        if !failure_records.is_empty() {
            self.write_records(&failure_records, "failures.jsonl")?;
        }

        // Write performance records
        if !performance_records.is_empty() {
            self.write_records(&performance_records, "performance.jsonl")?;
        }

        // Clear the records after flushing
        self.records.clear();

        Ok(())
    }

    /// Write a collection of records to a specific file.
    fn write_records(&self, records: &[&TelemetryRecord], filename: &str) -> anyhow::Result<()> {
        let file_path = self.base_dir.join(filename);
        
        // Read existing content if the file exists
        let mut content = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            String::new()
        };

        // Append new records
        for record in records {
            if let Ok(json_str) = serde_json::to_string(record) {
                content.push_str(&json_str);
                content.push_str("\n");
            }
        }

        // Write back to the file
        fs::write(&file_path, content)?;
        Ok(())
    }

    /// Generate a metrics report based on the collected telemetry data.
    pub fn generate_metrics_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Telemetry Metrics Report\n\n");

        // Count total events
        let total_events = self.records.len();
        report.push_str(&format!("## Summary\n\nTotal events: {}\n\n", total_events));

        // Count events by type
        let mut task_completed = 0;
        let mut task_failed = 0;
        let mut recovery_triggered = 0;
        let mut system_metrics = 0;

        for record in &self.records {
            match &record.event {
                TelemetryEvent::TaskCompleted { .. } => task_completed += 1,
                TelemetryEvent::TaskFailed { .. } => task_failed += 1,
                TelemetryEvent::RecoveryTriggered { .. } => recovery_triggered += 1,
                TelemetryEvent::SystemMetric { .. } => system_metrics += 1,
            }
        }

        report.push_str("## Event Breakdown\n\n");
        report.push_str(&format!("- Task completed: {}\n", task_completed));
        report.push_str(&format!("- Task failed: {}\n", task_failed));
        report.push_str(&format!("- Recovery triggered: {}\n", recovery_triggered));
        report.push_str(&format!("- System metrics: {}\n\n", system_metrics));

        // Calculate success rate
        if task_completed + task_failed > 0 {
            let success_rate = (task_completed as f64 / (task_completed + task_failed) as f64) * 100.0;
            report.push_str(&format!("## Success Rate\n\n{:.2}%\n\n", success_rate));
        }

        // Calculate average task duration
        let mut total_duration = 0;
        let mut duration_count = 0;

        for record in &self.records {
            match &record.event {
                TelemetryEvent::TaskCompleted { duration_ms, .. } => {
                    total_duration += duration_ms;
                    duration_count += 1;
                }
                TelemetryEvent::TaskFailed { duration_ms, .. } => {
                    total_duration += duration_ms;
                    duration_count += 1;
                }
                _ => {}
            }
        }

        if duration_count > 0 {
            let avg_duration = total_duration / duration_count;
            report.push_str(&format!("## Average Task Duration\n\n{} ms\n\n", avg_duration));
        }

        report
    }
}

impl Default for TelemetrySink {
    fn default() -> Self {
        Self::new(Path::new("."))
    }
}
