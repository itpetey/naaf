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
    pub runtime: WorkflowPackageRuntime,
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
        self.runtime.validate()?;
        self.ui.validate()?;
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
pub struct WorkflowPackageRuntime {
    #[serde(default)]
    pub llm: Option<WorkflowPackageLlmRuntime>,
    #[serde(default)]
    pub inputs: Vec<WorkflowPackageExecutionInput>,
}

impl WorkflowPackageRuntime {
    fn validate(&self) -> Result<()> {
        if let Some(llm) = &self.llm {
            llm.validate()?;
        }

        let mut input_ids = HashSet::new();
        for input in &self.inputs {
            input.validate()?;
            if !input_ids.insert(input.id.as_str()) {
                return Err(ValidationError::state(format!(
                    "duplicate workflow runtime input id '{}'",
                    input.id
                ))
                .into());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkflowPackageLlmRuntime {
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub providers: Vec<String>,
}

impl WorkflowPackageLlmRuntime {
    fn validate(&self) -> Result<()> {
        // Either providers OR model must be specified for required=true workflows
        // With no providers, host infers provider from model or user-configured API key
        // With no model, host uses default provider
        if self.required
            && self.providers.is_empty()
            && (self.model.is_empty() || self.model == "default")
        {
            return Err(ValidationError::state(
                "workflow runtime.llm requires provider or model to be specified",
            )
            .into());
        }

        let mut providers = HashSet::new();
        for provider in &self.providers {
            if provider.trim().is_empty() {
                return Err(ValidationError::state(
                    "workflow runtime.llm providers must not be empty",
                )
                .into());
            }
            if !providers.insert(provider.as_str()) {
                return Err(ValidationError::state(format!(
                    "duplicate workflow runtime provider '{}'",
                    provider
                ))
                .into());
            }
        }

        Ok(())
    }
}

fn default_model() -> String {
    "default".to_string()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkflowPackageExecutionInput {
    pub id: String,
    pub artifact: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub required: bool,
}

impl WorkflowPackageExecutionInput {
    fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(
                ValidationError::state("workflow runtime input id must not be empty").into(),
            );
        }
        if self.artifact.trim().is_empty() {
            return Err(ValidationError::state(format!(
                "workflow runtime input '{}' must declare an artifact key",
                self.id
            ))
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
    #[serde(default)]
    pub execution_guidance: String,
    #[serde(default)]
    pub primary_outputs: Vec<String>,
}

impl WorkflowPackageUi {
    fn validate(&self) -> Result<()> {
        let input_artifact = if self.input_artifact.trim().is_empty() {
            default_input_artifact()
        } else {
            self.input_artifact.clone()
        };

        if input_artifact.trim().is_empty() {
            return Err(ValidationError::state(
                "workflow package ui input_artifact must not be empty",
            )
            .into());
        }

        let mut primary_outputs = HashSet::new();
        for artifact in &self.primary_outputs {
            if artifact.trim().is_empty() {
                return Err(ValidationError::state(
                    "workflow package ui primary_outputs must not contain empty values",
                )
                .into());
            }
            if !primary_outputs.insert(artifact.as_str()) {
                return Err(ValidationError::state(format!(
                    "duplicate workflow primary output '{}'",
                    artifact
                ))
                .into());
            }
        }

        Ok(())
    }
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

[runtime.llm]
providers = ["mock"]

[[runtime.inputs]]
id = "repository_context"
artifact = "repository_context"
required = true

[ui]
input_artifact = "input"
input_prompt = "Describe the request"
execution_guidance = "Capture the user request and any repository context before execution."
primary_outputs = ["proposal", "acceptance"]

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
        assert_eq!(package.runtime.llm.unwrap().providers, vec!["mock"]);
        assert_eq!(package.runtime.inputs.len(), 1);
        assert_eq!(package.ui.primary_outputs, vec!["proposal", "acceptance"]);
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
    fn rejects_empty_runtime_provider_list() {
        let manifest = r#"
id = "broken"
name = "Broken"
entry = "start"

[runtime.llm]
required = true
providers = []

[[nodes]]
id = "start"
kind = "transformer"
step = "demo.step"
"#;

        let error = WorkflowPackage::from_toml_str(manifest).unwrap_err();
        assert!(error.to_string().contains("provider or model"));
    }

    #[test]
    fn rejects_duplicate_primary_outputs() {
        let manifest = r#"
id = "broken"
name = "Broken"
entry = "start"

[ui]
primary_outputs = ["result", "result"]

[[nodes]]
id = "start"
kind = "transformer"
step = "demo.step"
"#;

        let error = WorkflowPackage::from_toml_str(manifest).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("duplicate workflow primary output")
        );
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
