//! Orchestrator: workflow execution engine, run lifecycle, core contracts.
//!
//! # Legacy Code
//!
//! This module is part of the legacy prototype runtime.
//! **Do not build new features on this code.**
//! See the repository root `LEGACY.md` for details.
//! New development should target the new workflow runtime.

pub mod artifact;
pub mod finding;
pub mod graph;
pub mod journal;
pub mod remediation;
pub mod run;
pub mod store;
pub mod workflow;
