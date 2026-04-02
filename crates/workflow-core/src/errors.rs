use thiserror::Error;

use crate::events::EventError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Step error: {0}")]
    Step(#[from] StepError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Event error: {0}")]
    Event(#[from] EventError),
}

#[derive(Debug, Error)]
pub enum StepError {
    #[error("Transformer error in '{name}': {reason}")]
    Transformer { name: &'static str, reason: String },

    #[error("Router error in '{name}': {reason}")]
    Router { name: &'static str, reason: String },

    #[error("Reducer error in '{name}': {reason}")]
    Reducer { name: &'static str, reason: String },

    #[error("Step execution failed: {0}")]
    Execution(String),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Validation failed for '{name}': {reason}")]
    Validator { name: &'static str, reason: String },

    #[error("State validation failed: {0}")]
    State(String),
}

impl StepError {
    pub fn transformer(name: &'static str, reason: impl Into<String>) -> Self {
        Self::Transformer {
            name,
            reason: reason.into(),
        }
    }

    pub fn router(name: &'static str, reason: impl Into<String>) -> Self {
        Self::Router {
            name,
            reason: reason.into(),
        }
    }

    pub fn reducer(name: &'static str, reason: impl Into<String>) -> Self {
        Self::Reducer {
            name,
            reason: reason.into(),
        }
    }

    pub fn execution(reason: impl Into<String>) -> Self {
        Self::Execution(reason.into())
    }
}

impl ValidationError {
    pub fn validator(name: &'static str, reason: impl Into<String>) -> Self {
        Self::Validator {
            name,
            reason: reason.into(),
        }
    }

    pub fn state(reason: impl Into<String>) -> Self {
        Self::State(reason.into())
    }
}
