//! Overclock-AI Core — Orchestration engine for multi-AI agent coordination.
//!
//! This crate provides the core data models (tasks, roles, workflows),
//! context management, and event bus that power the overclock-ai orchestrator.
//!
//! # Design Principle
//!
//! Each AI CLI agent is treated as an **isolated worker** that:
//! - Receives a task description + context from the orchestrator
//! - Executes independently, unaware of other agents
//! - Returns structured output to the orchestrator
//!
//! The orchestrator is the **sole coordinator** — it manages task scheduling,
//! context synthesis, and result aggregation across all agents.

pub mod config;
pub mod context;
pub mod event;
pub mod recovery;
pub mod role;
pub mod task;
pub mod telemetry;
pub mod workflow;
pub mod validation;
