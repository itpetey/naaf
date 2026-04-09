use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Finding {
    pub class: FindingClass,
    pub severity: Severity,
    pub message: String,
    pub scope: Scope,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Scope {
    Global,
    File(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum FindingClass {
    TestFailure,
    DiffTooLarge,
    SchemaViolation,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}
