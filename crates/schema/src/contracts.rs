use serde::{Deserialize, Serialize};

use crate::artifacts::{ArtifactKey, ArtifactMap};
use crate::state_kind::StateKind;

/// Trait for adapting workflow outputs to match next workflow inputs.
pub trait WorkflowAdapter {
    fn adapt(&self, artifacts: &ArtifactMap) -> ArtifactMap;
}

/// Pass-through adapter that returns artifacts unchanged.
pub struct IdentityAdapter;

impl WorkflowAdapter for IdentityAdapter {
    fn adapt(&self, artifacts: &ArtifactMap) -> ArtifactMap {
        artifacts.clone()
    }
}

/// Adapter that renames artifact keys.
///
/// Only artifacts whose keys are in the mappings are preserved;
/// all other artifacts are dropped from the output.
pub struct RenameAdapter {
    pub mappings: Vec<(ArtifactKey, ArtifactKey)>,
}

impl WorkflowAdapter for RenameAdapter {
    fn adapt(&self, artifacts: &ArtifactMap) -> ArtifactMap {
        let mut result = ArtifactMap::new();
        for (old_key, new_key) in &self.mappings {
            if let Some(value) = artifacts.get(old_key) {
                result.insert(new_key.clone(), value.clone());
            }
        }
        result
    }
}

impl RenameAdapter {
    pub fn new(mappings: Vec<(ArtifactKey, ArtifactKey)>) -> Self {
        Self { mappings }
    }
}

/// Contract declaring what a workflow accepts and guarantees.
///
/// This enables composition without structural coupling by making
/// workflow interfaces explicit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkflowContract {
    /// State kinds this workflow accepts as input.
    pub accepted_kinds: Vec<StateKind>,
    /// Artifacts this workflow requires as input.
    pub required_artifacts: Vec<ArtifactKey>,
    /// Artifacts this workflow guarantees to produce.
    pub guaranteed_artifacts: Vec<ArtifactKey>,
    /// State kinds this workflow may output.
    pub possible_output_kinds: Vec<StateKind>,
}

impl WorkflowContract {
    pub fn new(
        accepted_kinds: Vec<StateKind>,
        required_artifacts: Vec<ArtifactKey>,
        guaranteed_artifacts: Vec<ArtifactKey>,
        possible_output_kinds: Vec<StateKind>,
    ) -> Self {
        Self {
            accepted_kinds,
            required_artifacts,
            guaranteed_artifacts,
            possible_output_kinds,
        }
    }
}

/// Checks whether one workflow's output can feed into another's input.
///
/// Returns true if:
/// - All source guaranteed_artifacts are in target required_artifacts
/// - All source possible_output_kinds are in target accepted_kinds
///
/// Empty collections use vacuous truth:
/// - Empty guaranteed_artifacts always satisfies artifact requirements
/// - Empty possible_output_kinds always satisfies kind requirements
pub fn is_compatible(source: &WorkflowContract, target: &WorkflowContract) -> bool {
    source
        .guaranteed_artifacts
        .iter()
        .all(|artifact| target.required_artifacts.contains(artifact))
        && source
            .possible_output_kinds
            .iter()
            .all(|kind| target.accepted_kinds.contains(kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactValue;

    #[test]
    fn test_workflow_contract_creation() {
        let contract = WorkflowContract::new(
            vec![StateKind::Proposed, StateKind::Normalized],
            vec![ArtifactKey::new("input")],
            vec![ArtifactKey::new("output")],
            vec![StateKind::Planned],
        );
        assert_eq!(contract.accepted_kinds.len(), 2);
        assert_eq!(contract.required_artifacts.len(), 1);
        assert_eq!(contract.guaranteed_artifacts.len(), 1);
        assert_eq!(contract.possible_output_kinds.len(), 1);
    }

    #[test]
    fn test_workflow_contract_serde() {
        let contract = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![ArtifactKey::new("input")],
            vec![ArtifactKey::new("output")],
            vec![StateKind::Normalized],
        );
        let json = serde_json::to_string(&contract).unwrap();
        let restored: WorkflowContract = serde_json::from_str(&json).unwrap();
        assert_eq!(contract.accepted_kinds, restored.accepted_kinds);
        assert_eq!(contract.required_artifacts, restored.required_artifacts);
    }

    #[test]
    fn test_is_compatible_full_match() {
        let source = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![],
            vec![ArtifactKey::new("result")],
            vec![StateKind::Normalized],
        );
        let target = WorkflowContract::new(
            vec![StateKind::Normalized],
            vec![ArtifactKey::new("result")],
            vec![],
            vec![StateKind::Planned],
        );
        assert!(is_compatible(&source, &target));
    }

    #[test]
    fn test_is_compatible_partial_match() {
        let source = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![],
            vec![ArtifactKey::new("result")],
            vec![StateKind::Normalized, StateKind::Planned],
        );
        let target = WorkflowContract::new(
            vec![StateKind::Normalized],
            vec![ArtifactKey::new("result")],
            vec![],
            vec![StateKind::Planned],
        );
        assert!(!is_compatible(&source, &target));
    }

    #[test]
    fn test_is_compatible_multiple_kinds_all_accepted() {
        let source = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![],
            vec![ArtifactKey::new("result")],
            vec![StateKind::Normalized, StateKind::Planned],
        );
        let target = WorkflowContract::new(
            vec![
                StateKind::Normalized,
                StateKind::Planned,
                StateKind::Accepted,
            ],
            vec![ArtifactKey::new("result")],
            vec![],
            vec![StateKind::Accepted],
        );
        assert!(is_compatible(&source, &target));
    }

    #[test]
    fn test_is_compatible_no_match_artifacts() {
        let source = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![],
            vec![ArtifactKey::new("wrong")],
            vec![StateKind::Normalized],
        );
        let target = WorkflowContract::new(
            vec![StateKind::Normalized],
            vec![ArtifactKey::new("result")],
            vec![],
            vec![StateKind::Planned],
        );
        assert!(!is_compatible(&source, &target));
    }

    #[test]
    fn test_is_compatible_no_match_kinds() {
        let source = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![],
            vec![ArtifactKey::new("result")],
            vec![StateKind::Planned],
        );
        let target = WorkflowContract::new(
            vec![StateKind::Normalized],
            vec![ArtifactKey::new("result")],
            vec![],
            vec![StateKind::Planned],
        );
        assert!(!is_compatible(&source, &target));
    }

    #[test]
    fn test_is_compatible_empty_guaranteed() {
        let source = WorkflowContract::new(
            vec![StateKind::Proposed],
            vec![],
            vec![],
            vec![StateKind::Normalized],
        );
        let target = WorkflowContract::new(
            vec![StateKind::Normalized],
            vec![ArtifactKey::new("result")],
            vec![],
            vec![StateKind::Planned],
        );
        assert!(is_compatible(&source, &target));
    }

    #[test]
    fn test_identity_adapter() {
        let mut artifacts = ArtifactMap::new();
        artifacts.insert(ArtifactKey::new("key1"), ArtifactValue::text("value1"));
        artifacts.insert(ArtifactKey::new("key2"), ArtifactValue::text("value2"));

        let adapter = IdentityAdapter;
        let result = adapter.adapt(&artifacts);

        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get_text(&ArtifactKey::new("key1")),
            Some(&"value1".to_string())
        );
    }

    #[test]
    fn test_rename_adapter() {
        let mut artifacts = ArtifactMap::new();
        artifacts.insert(ArtifactKey::new("old1"), ArtifactValue::text("value1"));
        artifacts.insert(ArtifactKey::new("old2"), ArtifactValue::text("value2"));
        artifacts.insert(ArtifactKey::new("unrelated"), ArtifactValue::text("value3"));

        let adapter = RenameAdapter::new(vec![
            (ArtifactKey::new("old1"), ArtifactKey::new("new1")),
            (ArtifactKey::new("old2"), ArtifactKey::new("new2")),
        ]);

        let result = adapter.adapt(&artifacts);

        assert_eq!(result.len(), 2);
        assert!(result.get(&ArtifactKey::new("new1")).is_some());
        assert!(result.get(&ArtifactKey::new("new2")).is_some());
        assert!(result.get(&ArtifactKey::new("old1")).is_none());
        assert!(result.get(&ArtifactKey::new("unrelated")).is_none());
    }
}
