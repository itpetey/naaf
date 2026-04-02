use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ArtifactKey(String);

impl ArtifactKey {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl Serialize for ArtifactKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ArtifactKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

impl std::fmt::Display for ArtifactKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum ArtifactValue {
    Text(String),
    Json(serde_json::Value),
}

impl ArtifactValue {
    pub fn as_text(&self) -> Option<&String> {
        match self {
            Self::Text(s) => Some(s),
            Self::Json(_) => None,
        }
    }

    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Text(_) => None,
            Self::Json(v) => Some(v),
        }
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn json(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactMap(HashMap<ArtifactKey, ArtifactValue>);

impl ArtifactMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, key: ArtifactKey, value: ArtifactValue) {
        self.0.insert(key, value);
    }

    pub fn get(&self, key: &ArtifactKey) -> Option<&ArtifactValue> {
        self.0.get(key)
    }

    pub fn get_text(&self, key: &ArtifactKey) -> Option<&String> {
        self.get(key).and_then(|v| v.as_text())
    }

    pub fn get_json(&self, key: &ArtifactKey) -> Option<&serde_json::Value> {
        self.get(key).and_then(|v| v.as_json())
    }

    pub fn contains_key(&self, key: &ArtifactKey) -> bool {
        self.0.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ArtifactKey, &ArtifactValue)> {
        self.0.iter()
    }
}
