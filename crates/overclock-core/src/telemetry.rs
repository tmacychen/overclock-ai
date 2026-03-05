//! Telemetry and Data Collection
//!
//! Based on ADDS v2.1 specifications, this module is responsible for capturing
//! success, failure, and performance data points, saving them locally for future
//! model fine-tuning and process evaluation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

/// A rudimentary local sink for collecting and formatting telemetry data (e.g. JSONL)
/// Future implementation will write these directly to `.ai/training_data/` directory.
pub struct TelemetrySink {
    // In memory store for now; can be flushed to disk
    pub records: Vec<TelemetryRecord>,
}

impl TelemetrySink {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
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
}

impl Default for TelemetrySink {
    fn default() -> Self {
        Self::new()
    }
}
