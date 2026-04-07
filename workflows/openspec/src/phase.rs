//! Phase definitions for OpenSpec workflow.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Phase {
    #[default]
    Proposed,
    Normalized,
    Scoped,
    Planned,
    ReadyForPlanning,
    ReadyForImplementation,
    ReadyForValidation,
    ReadyForReview,
    ReadyForRemediation,
    Accepted,
    Terminal,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Proposed => write!(f, "Proposed"),
            Phase::Normalized => write!(f, "Normalized"),
            Phase::Scoped => write!(f, "Scoped"),
            Phase::Planned => write!(f, "Planned"),
            Phase::ReadyForPlanning => write!(f, "ReadyForPlanning"),
            Phase::ReadyForImplementation => write!(f, "ReadyForImplementation"),
            Phase::ReadyForValidation => write!(f, "ReadyForValidation"),
            Phase::ReadyForReview => write!(f, "ReadyForReview"),
            Phase::ReadyForRemediation => write!(f, "ReadyForRemediation"),
            Phase::Accepted => write!(f, "Accepted"),
            Phase::Terminal => write!(f, "Terminal"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_display() {
        assert_eq!(Phase::Proposed.to_string(), "Proposed");
        assert_eq!(Phase::Normalized.to_string(), "Normalized");
        assert_eq!(Phase::Accepted.to_string(), "Accepted");
    }

    #[test]
    fn test_phase_default() {
        let phase: Phase = Default::default();
        assert_eq!(phase, Phase::Proposed);
    }
}
