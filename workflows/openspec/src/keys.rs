//! Type-safe artifact key constants.
//!
//! This module provides constants for commonly used artifact keys
//! to prevent typos and improve code maintainability.

use naaf_schema::artifacts::ArtifactKey;

/// Artifact keys for the draft_request workflow pipeline.
pub struct DraftRequestKeys;

impl DraftRequestKeys {
    /// Key for raw user input text.
    pub const INPUT: &'static str = "input";

    /// Key for proposal artifact (Proposal struct).
    pub const PROPOSAL: &'static str = "proposal";

    /// Key for classification result (Classification struct).
    pub const CLASSIFICATION: &'static str = "classification";

    /// Key for normalized input (NormalizedInput struct).
    pub const NORMALIZED: &'static str = "normalized";

    /// Key for scope analysis (ScopeAnalysis struct).
    pub const SCOPE: &'static str = "scope";

    /// Key for execution plan (Plan struct).
    pub const PLAN: &'static str = "plan";

    /// Key for acceptance result (Acceptance struct).
    pub const ACCEPTANCE: &'static str = "acceptance";

    /// Key for terminal response text.
    pub const RESPONSE: &'static str = "response";

    /// Key for escalation metadata.
    pub const ESCALATION: &'static str = "escalation";

    /// Creates an ArtifactKey for input.
    pub fn input() -> ArtifactKey {
        ArtifactKey::new(Self::INPUT)
    }

    /// Creates an ArtifactKey for proposal.
    pub fn proposal() -> ArtifactKey {
        ArtifactKey::new(Self::PROPOSAL)
    }

    /// Creates an ArtifactKey for classification.
    pub fn classification() -> ArtifactKey {
        ArtifactKey::new(Self::CLASSIFICATION)
    }

    /// Creates an ArtifactKey for normalized.
    pub fn normalized() -> ArtifactKey {
        ArtifactKey::new(Self::NORMALIZED)
    }

    /// Creates an ArtifactKey for scope.
    pub fn scope() -> ArtifactKey {
        ArtifactKey::new(Self::SCOPE)
    }

    /// Creates an ArtifactKey for plan.
    pub fn plan() -> ArtifactKey {
        ArtifactKey::new(Self::PLAN)
    }

    /// Creates an ArtifactKey for acceptance.
    pub fn acceptance() -> ArtifactKey {
        ArtifactKey::new(Self::ACCEPTANCE)
    }

    /// Creates an ArtifactKey for response.
    pub fn response() -> ArtifactKey {
        ArtifactKey::new(Self::RESPONSE)
    }

    /// Creates an ArtifactKey for escalation.
    pub fn escalation() -> ArtifactKey {
        ArtifactKey::new(Self::ESCALATION)
    }
}

/// Alias for backward compatibility.
pub type ClassificationKeys = DraftRequestKeys;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_constants() {
        assert_eq!(DraftRequestKeys::INPUT, "input");
        assert_eq!(DraftRequestKeys::PROPOSAL, "proposal");
        assert_eq!(DraftRequestKeys::CLASSIFICATION, "classification");
        assert_eq!(DraftRequestKeys::NORMALIZED, "normalized");
        assert_eq!(DraftRequestKeys::SCOPE, "scope");
        assert_eq!(DraftRequestKeys::PLAN, "plan");
        assert_eq!(DraftRequestKeys::ACCEPTANCE, "acceptance");
        assert_eq!(DraftRequestKeys::RESPONSE, "response");
        assert_eq!(DraftRequestKeys::ESCALATION, "escalation");
    }

    #[test]
    fn test_key_constructors() {
        let input_key = DraftRequestKeys::input();
        assert_eq!(input_key.to_string(), "input");

        let proposal_key = DraftRequestKeys::proposal();
        assert_eq!(proposal_key.to_string(), "proposal");

        let classification_key = DraftRequestKeys::classification();
        assert_eq!(classification_key.to_string(), "classification");

        let normalized_key = DraftRequestKeys::normalized();
        assert_eq!(normalized_key.to_string(), "normalized");
    }

    #[test]
    fn test_backward_compatibility() {
        // ClassificationKeys should still work
        assert_eq!(ClassificationKeys::INPUT, "input");
        assert_eq!(ClassificationKeys::CLASSIFICATION, "classification");
    }
}
