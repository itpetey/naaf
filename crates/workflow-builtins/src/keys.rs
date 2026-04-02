//! Type-safe artifact key constants.
//!
//! This module provides constants for commonly used artifact keys
//! to prevent typos and improve code maintainability.

use workflow_schema::artifacts::ArtifactKey;

/// Artifact keys for classification-related data.
pub struct ClassificationKeys;

impl ClassificationKeys {
    /// Key for raw user input text.
    pub const INPUT: &'static str = "input";

    /// Key for classification result (Classification struct).
    pub const CLASSIFICATION: &'static str = "classification";

    /// Key for terminal response text.
    pub const RESPONSE: &'static str = "response";

    /// Key for escalation metadata.
    pub const ESCALATION: &'static str = "escalation";

    /// Creates an ArtifactKey for input.
    pub fn input() -> ArtifactKey {
        ArtifactKey::new(Self::INPUT)
    }

    /// Creates an ArtifactKey for classification.
    pub fn classification() -> ArtifactKey {
        ArtifactKey::new(Self::CLASSIFICATION)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_constants() {
        assert_eq!(ClassificationKeys::INPUT, "input");
        assert_eq!(ClassificationKeys::CLASSIFICATION, "classification");
        assert_eq!(ClassificationKeys::RESPONSE, "response");
        assert_eq!(ClassificationKeys::ESCALATION, "escalation");
    }

    #[test]
    fn test_key_constructors() {
        let input_key = ClassificationKeys::input();
        assert_eq!(input_key.to_string(), "input");

        let classification_key = ClassificationKeys::classification();
        assert_eq!(classification_key.to_string(), "classification");
    }
}
