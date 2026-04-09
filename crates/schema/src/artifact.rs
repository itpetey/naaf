use std::{any::Any, sync::Arc};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait Artifact {
    fn kind(&self) -> ArtifactKind;
}

pub trait ErasedArtifact: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn serialize(&self) -> serde_json::Value;
}

#[derive(Clone)]
pub struct ArtifactRef {
    pub id: Uuid,
    pub kind: ArtifactKind,
    pub produced_by: TransitionId,
    pub parents: Vec<ArtifactId>,
    pub data: Arc<dyn ErasedArtifact>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ArtifactMeta {
    pub id: Uuid,
    pub kind: ArtifactKind,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ArtifactKind {
    RawRequest,
    ClarifiedRequest,
    FeatureSpec,
    PatchSet,
    TestResults,
    FindingSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransitionId(pub String);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactRef {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.data.as_any().downcast_ref::<T>()
    }
}

impl TransitionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl<T> ErasedArtifact for T
where
    T: 'static + Send + Sync + Serialize,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn serialize(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}
