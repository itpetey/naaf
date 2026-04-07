use std::path::Path;

use crate::budget::Services;
use crate::errors::{Result, ValidationError};
use crate::graph::{CompiledWorkflow, EdgeType, GraphEdge};
use crate::workflow_package::{DiscoveredWorkflowPackage, WorkflowEdgeKind, WorkflowPackage};
use crate::workflow_registry::WorkflowRegistry;
use tracing::warn;

pub fn discover_workflow_packages(root_dir: &Path) -> Result<Vec<DiscoveredWorkflowPackage>> {
    if !root_dir.exists() {
        return Ok(Vec::new());
    }

    let mut discovered = Vec::new();
    for entry in std::fs::read_dir(root_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("workflow.toml");
        if !manifest_path.exists() {
            continue;
        }

        match WorkflowPackage::from_path(&manifest_path) {
            Ok(package) => discovered.push(DiscoveredWorkflowPackage {
                root_dir: path,
                manifest_path,
                package,
            }),
            Err(error) => {
                warn!(
                    manifest = %manifest_path.display(),
                    error = %error,
                    "Skipping invalid workflow package manifest"
                );
            }
        }
    }

    discovered.sort_by(|left, right| left.package.id.cmp(&right.package.id));
    Ok(discovered)
}

pub fn build_workflow<S: Services>(
    package: &WorkflowPackage,
    registry: &WorkflowRegistry<S>,
) -> Result<CompiledWorkflow<S>> {
    package.validate()?;

    let mut workflow = CompiledWorkflow::new(&package.id, &package.entry);
    for node in &package.nodes {
        workflow.add_node(registry.build_node(node)?);
    }
    for edge in &package.edges {
        let graph_edge = match edge.kind {
            WorkflowEdgeKind::Normal => GraphEdge::new(&edge.from, &edge.to),
            WorkflowEdgeKind::Conditional => GraphEdge::conditional(&edge.from, &edge.to),
            WorkflowEdgeKind::Join => GraphEdge::join(&edge.from, &edge.to),
        };
        workflow.add_edge(graph_edge);
    }

    workflow.validate()?;
    workflow
        .topological_sort()
        .map_err(crate::errors::Error::from)?;
    validate_join_targets(&workflow)?;
    Ok(workflow)
}

fn validate_join_targets<S: Services>(workflow: &CompiledWorkflow<S>) -> Result<()> {
    for edge in &workflow.edges {
        if edge.edge_type != EdgeType::Join {
            continue;
        }

        let Some(target) = workflow.get_node(&edge.target) else {
            return Err(ValidationError::state(format!(
                "join edge references missing node '{}'",
                edge.target
            ))
            .into());
        };

        if !matches!(target, crate::graph::GraphNode::Reducer { .. }) {
            return Err(ValidationError::state(format!(
                "join edge '{}' -> '{}' must target a reducer",
                edge.source, edge.target
            ))
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::DummyServices;
    use crate::errors::StepError;
    use crate::steps::Transformer;
    use tempfile::tempdir;

    struct TestTransformer;

    impl Transformer for TestTransformer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "test_transformer"
        }

        fn transform(
            &self,
            _ctx: &mut crate::budget::ExecCtx<Self::Services>,
            input: naaf_schema::state::StateEnvelope,
        ) -> std::result::Result<naaf_schema::state::StateEnvelope, StepError> {
            Ok(input)
        }
    }

    #[test]
    fn discovers_workflow_manifests_from_directory() {
        let temp = tempdir().unwrap();
        let workflow_dir = temp.path().join("demo");
        std::fs::create_dir_all(&workflow_dir).unwrap();
        std::fs::write(
            workflow_dir.join("workflow.toml"),
            r#"
id = "demo"
name = "Demo"
entry = "start"

[[nodes]]
id = "start"
kind = "transformer"
step = "demo.step"
"#,
        )
        .unwrap();

        let packages = discover_workflow_packages(temp.path()).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].package.id, "demo");
    }

    #[test]
    fn ignores_invalid_workflow_manifest() {
        let temp = tempdir().unwrap();

        let valid_dir = temp.path().join("valid");
        std::fs::create_dir_all(&valid_dir).unwrap();
        std::fs::write(
            valid_dir.join("workflow.toml"),
            r#"
id = "valid"
name = "Valid"
entry = "start"

[[nodes]]
id = "start"
kind = "transformer"
step = "demo.step"
"#,
        )
        .unwrap();

        let invalid_dir = temp.path().join("invalid");
        std::fs::create_dir_all(&invalid_dir).unwrap();
        std::fs::write(invalid_dir.join("workflow.toml"), "id = 42").unwrap();

        let packages = discover_workflow_packages(temp.path()).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].package.id, "valid");
    }

    #[test]
    fn builds_workflow_from_package_and_registry() {
        let package = WorkflowPackage::from_toml_str(
            r#"
id = "demo"
name = "Demo"
entry = "start"

[[nodes]]
id = "start"
kind = "transformer"
step = "demo.step"
"#,
        )
        .unwrap();

        let mut registry = WorkflowRegistry::<DummyServices>::new();
        registry.register_transformer("demo.step", |_| {
            Ok(crate::steps::BoxedTransformer::new(TestTransformer))
        });

        let workflow = build_workflow(&package, &registry).unwrap();
        assert_eq!(workflow.name, "demo");
        assert_eq!(workflow.entry_point, "start");
        assert_eq!(workflow.nodes.len(), 1);
    }
}
