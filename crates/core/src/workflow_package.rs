use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result, ValidationError};

fn default_input_artifact() -> String {
    "input".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkflowPackage {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub entry: String,
    #[serde(default)]
    pub ui: WorkflowPackageUi,
    pub nodes: Vec<WorkflowPackageNode>,
    #[serde(default)]
    pub edges: Vec<WorkflowPackageEdge>,
}

impl WorkflowPackage {
    pub fn from_toml_str(input: &str) -> Result<Self> {
        let package: Self = toml::from_str(input)
            .map_err(|err| Error::WorkflowPackage(format!("Failed to parse manifest: {err}")))?;
        package.validate()?;
        Ok(package)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let input = std::fs::read_to_string(path)?;
        Self::from_toml_str(&input)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(ValidationError::state("workflow package id must not be empty").into());
        }
        if self.name.trim().is_empty() {
            return Err(ValidationError::state("workflow package name must not be empty").into());
        }
        if self.entry.trim().is_empty() {
            return Err(ValidationError::state("workflow package entry must not be empty").into());
        }
        if self.nodes.is_empty() {
            return Err(
                ValidationError::state("workflow package must define at least one node").into(),
            );
        }

        let mut node_ids = HashSet::new();
        for node in &self.nodes {
            node.validate()?;
            if !node_ids.insert(node.id.as_str()) {
                return Err(ValidationError::state(format!(
                    "duplicate workflow node id '{}'",
                    node.id
                ))
                .into());
            }
        }

        if !node_ids.contains(self.entry.as_str()) {
            return Err(ValidationError::state(format!(
                "workflow entry '{}' not found in node list",
                self.entry
            ))
            .into());
        }

        for edge in &self.edges {
            edge.validate(&node_ids, &self.nodes)?;
        }

        let has_terminal = self
            .nodes
            .iter()
            .any(|node| !self.edges.iter().any(|edge| edge.from == node.id));
        if !has_terminal {
            return Err(ValidationError::state(
                "workflow package must contain a terminal node with no outgoing edges",
            )
            .into());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkflowPackageUi {
    #[serde(default = "default_input_artifact")]
    pub input_artifact: String,
    #[serde(default)]
    pub input_prompt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkflowPackageNode {
    pub id: String,
    pub kind: WorkflowNodeKind,
    pub step: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

impl WorkflowPackageNode {
    fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(ValidationError::state("workflow node id must not be empty").into());
        }
        if self.step.trim().is_empty() {
            return Err(ValidationError::state(format!(
                "workflow node '{}' must reference a step kind",
                self.id
            ))
            .into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    Transformer,
    Router,
    Reducer,
    Validator,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEdgeKind {
    #[default]
    Normal,
    Conditional,
    Join,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkflowPackageEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub kind: WorkflowEdgeKind,
}

impl WorkflowPackageEdge {
    fn validate(&self, node_ids: &HashSet<&str>, nodes: &[WorkflowPackageNode]) -> Result<()> {
        if !node_ids.contains(self.from.as_str()) {
            return Err(ValidationError::state(format!(
                "workflow edge references unknown source node '{}'",
                self.from
            ))
            .into());
        }
        if !node_ids.contains(self.to.as_str()) {
            return Err(ValidationError::state(format!(
                "workflow edge references unknown target node '{}'",
                self.to
            ))
            .into());
        }

        if self.kind == WorkflowEdgeKind::Join {
            let target = nodes
                .iter()
                .find(|node| node.id == self.to)
                .ok_or_else(|| ValidationError::state("join edge target missing from node list"))?;
            if target.kind != WorkflowNodeKind::Reducer {
                return Err(ValidationError::state(format!(
                    "join edge '{}' -> '{}' must target a reducer node",
                    self.from, self.to
                ))
                .into());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DiscoveredWorkflowPackage {
    pub root_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub package: WorkflowPackage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_manifest() {
        let manifest = r#"
id = "draft-request"
name = "Draft Request"
summary = "Structured request drafting"
entry = "propose"

[ui]
input_artifact = "input"
input_prompt = "Describe the request"

[[nodes]]
id = "propose"
kind = "transformer"
step = "openspec.propose"

[[nodes]]
id = "done"
kind = "validator"
step = "openspec.done"

[[edges]]
from = "propose"
to = "done"
"#;

        let package = WorkflowPackage::from_toml_str(manifest).unwrap();
        assert_eq!(package.id, "draft-request");
        assert_eq!(package.ui.input_artifact, "input");
    }

    #[test]
    fn rejects_unknown_entry() {
        let manifest = r#"
id = "broken"
name = "Broken"
entry = "missing"

[[nodes]]
id = "propose"
kind = "transformer"
step = "openspec.propose"
"#;

        let error = WorkflowPackage::from_toml_str(manifest).unwrap_err();
        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn rejects_join_edge_to_non_reducer() {
        let manifest = r#"
id = "broken"
name = "Broken"
entry = "start"

[[nodes]]
id = "start"
kind = "router"
step = "route.start"

[[nodes]]
id = "end"
kind = "validator"
step = "done"

[[edges]]
from = "start"
to = "end"
kind = "join"
"#;

        let error = WorkflowPackage::from_toml_str(manifest).unwrap_err();
        assert!(error.to_string().contains("must target a reducer node"));
    }
}
