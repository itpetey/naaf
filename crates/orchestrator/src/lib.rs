//! Orchestrator: workflow execution engine, run lifecycle, core contracts.
//!
//! **DEPRECATED - MIGRATION IN PROGRESS**
//!
//! This module is part of the legacy prototype runtime.
//! **Do not build new features on this code.**
//!
//! # Migration Status
//!
//! Artifact schemas and LLM prompts have been migrated to the new runtime:
//! - Artifacts: `workflow_schema` crate (re-exported via `naaf_openspec`)
//! - Prompts: `workflow_llm::prompts` module
//! - LLM Steps: `workflow_builtins::llm_steps` module
//!
//! # Legacy Components
//!
//! Remaining in this crate:
//! - `DefaultExecutionEngine` - Legacy workflow executor
//! - `ArtifactStore` - File-based artifact storage  
//! - `Journal` - Event logging
//! - `Phase`/`Run` state management
//!
//! # New Runtime
//!
//! Use these crates for new development:
//! - `workflow-core` - Modern execution engine with `Executor` and `WorkflowBuilder`
//! - `workflow-schema` - State management with `StateEnvelope` and artifacts
//! - `workflow-builtins` - Reusable step implementations
//!
//! See `LEGACY.md` and `MIGRATION.md` for details.

pub mod artifact;
pub mod finding;
pub mod graph;
pub mod journal;
pub mod remediation;
pub mod run;
pub mod store;
pub mod workflow;
