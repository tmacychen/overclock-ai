//! AI CLI tool adapters for overclock-ai.
//!
//! Each adapter wraps a specific AI CLI tool (CodeBuddy, Kiro CLI, Trae Agent, etc.)
//! behind a unified `AgentAdapter` trait. The orchestrator interacts with all agents
//! through this trait, never through tool-specific APIs.
//!
//! # Agent Isolation Principle
//!
//! Each CLI agent is **unaware of other agents**. It receives:
//! 1. A task description (what to do)
//! 2. A context prompt (project info + upstream task results)
//! 3. A working directory
//!
//! The adapter is responsible for:
//! - Converting the unified context into the CLI's native format
//! - Invoking the CLI tool as a subprocess
//! - Parsing the CLI's output into a unified `TaskOutput`
//! - Streaming real-time output events to the event bus

pub mod adapter_trait;
pub mod codebuddy;
pub mod custom_api;
pub mod gemini;
pub mod kiro;

pub use adapter_trait::{AgentAdapter, HealthStatus, TaskOutput};
