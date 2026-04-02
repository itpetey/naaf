use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::artifacts::ArtifactMap;
use crate::lineage::Lineage;
use crate::meta::StateMeta;
use crate::state_kind::StateKind;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId(Uuid);

impl StateId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StateId {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for StateId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for StateId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let uuid = Uuid::parse_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Self(uuid))
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RunId(Uuid);

impl RunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for RunId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for RunId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let uuid = Uuid::parse_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Self(uuid))
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StateEnvelope {
    pub id: StateId,
    pub run_id: RunId,
    pub kind: StateKind,
    pub artifacts: ArtifactMap,
    pub meta: StateMeta,
    pub lineage: Lineage,
}

impl StateEnvelope {
    pub fn new(id: StateId, run_id: RunId, kind: StateKind, lineage: Lineage) -> Self {
        Self {
            id,
            run_id,
            kind,
            artifacts: ArtifactMap::new(),
            meta: StateMeta::now(),
            lineage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_id_generation() {
        let id = StateId::new();
        let id2 = StateId::new();
        assert_ne!(id, id2);
    }

    #[test]
    fn test_run_id_generation() {
        let id = RunId::new();
        let id2 = RunId::new();
        assert_ne!(id, id2);
    }

    #[test]
    fn test_state_kind_variants() {
        let kinds = [
            StateKind::Proposed,
            StateKind::Normalized,
            StateKind::Scoped,
            StateKind::Planned,
            StateKind::Accepted,
            StateKind::Ambiguous,
            StateKind::Escalated,
            StateKind::Terminal,
        ];
        assert_eq!(kinds.len(), 8);
    }

    #[test]
    fn test_state_envelope_creation() {
        let envelope = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Proposed,
            Lineage::new(None, None),
        );
        assert_eq!(envelope.kind, StateKind::Proposed);
        assert!(envelope.artifacts.is_empty());
    }

    #[test]
    fn test_state_envelope_serde() {
        let envelope = StateEnvelope::new(
            StateId::new(),
            RunId::new(),
            StateKind::Normalized,
            Lineage::new(None, Some("normalize".to_string())),
        );
        let json = serde_json::to_string(&envelope).unwrap();
        let restored: StateEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(envelope.id, restored.id);
        assert_eq!(envelope.kind, restored.kind);
    }
}
