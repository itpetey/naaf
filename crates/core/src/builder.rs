use std::collections::HashMap;

use crate::budget::Services;
use crate::errors::{Result, ValidationError};
use crate::graph::{CompiledWorkflow, EdgeType, GraphEdge, GraphNode};
use crate::steps::{BoxedReducer, BoxedRouter, BoxedTransformer, BoxedValidator};

pub struct WorkflowBuilder<S: Services> {
    name: String,
    nodes: Vec<GraphNode<S>>,
    edges: Vec<GraphEdge>,
    step_ids: HashMap<String, usize>,
    entry_point: Option<String>,
}

impl<S: Services> WorkflowBuilder<S> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            step_ids: HashMap::new(),
            entry_point: None,
        }
    }

    pub fn step(mut self, id: impl Into<String>, transformer: BoxedTransformer<S>) -> Self {
        let id = id.into();
        let node_index = self.nodes.len();
        let node = GraphNode::transformer(&id, transformer);
        self.step_ids.insert(id.clone(), node_index);
        self.nodes.push(node);
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
        }
        self
    }

    pub fn route(mut self, id: impl Into<String>, router: BoxedRouter<S>) -> Self {
        let id = id.into();
        let node_index = self.nodes.len();
        let node = GraphNode::router(&id, router);
        self.step_ids.insert(id.clone(), node_index);
        self.nodes.push(node);
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
        }
        self
    }

    pub fn branch(mut self, id: impl Into<String>, validator: BoxedValidator<S>) -> Self {
        let id = id.into();
        let node_index = self.nodes.len();
        let node = GraphNode::validator(&id, validator);
        self.step_ids.insert(id.clone(), node_index);
        self.nodes.push(node);
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
        }
        self
    }

    pub fn path(mut self, id: impl Into<String>, step: impl Into<String>) -> Self {
        let id = id.into();
        let step = step.into();
        let edge = GraphEdge::new(&id, &step);
        self.edges.push(edge);
        self
    }

    pub fn join(mut self, id: impl Into<String>, reducer: BoxedReducer<S>) -> Self {
        let id = id.into();
        let node_index = self.nodes.len();
        let node = GraphNode::reducer(&id, reducer);
        self.step_ids.insert(id.clone(), node_index);
        self.nodes.push(node);
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
        }
        self
    }

    pub fn terminal(mut self, id: impl Into<String>, validator: BoxedValidator<S>) -> Self {
        let id = id.into();
        let node_index = self.nodes.len();
        let node = GraphNode::validator(&id, validator);
        self.step_ids.insert(id.clone(), node_index);
        self.nodes.push(node);
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
        }
        self
    }

    pub fn compile(self) -> Result<CompiledWorkflow<S>> {
        self.validate()?;
        let entry_point = self
            .entry_point
            .ok_or_else(|| ValidationError::state("no entry point defined"))?;
        let mut workflow = CompiledWorkflow::new(&self.name, &entry_point);
        for node in self.nodes {
            workflow.add_node(node);
        }
        for edge in self.edges {
            workflow.add_edge(edge);
        }
        workflow.validate()?;
        Ok(workflow)
    }

    fn validate(&self) -> Result<()> {
        self.validate_unique_step_ids()?;
        self.validate_references()?;
        self.validate_terminal_path()?;
        self.validate_join_reducer()?;
        self.validate_acyclicity()?;
        Ok(())
    }

    fn validate_unique_step_ids(&self) -> Result<()> {
        let mut seen: HashMap<String, usize> = HashMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let id = node.id();
            if let Some(existing_index) = seen.insert(id.to_string(), index) {
                return Err(ValidationError::state(format!(
                    "duplicate step ID: '{}' already exists at index {}",
                    id, existing_index
                ))
                .into());
            }
        }
        Ok(())
    }

    fn validate_references(&self) -> Result<()> {
        let node_ids: std::collections::HashSet<_> = self.nodes.iter().map(|n| n.id()).collect();
        for edge in &self.edges {
            if !node_ids.contains(edge.source.as_str()) {
                return Err(ValidationError::state(format!(
                    "edge references non-existent source node: '{}'",
                    edge.source
                ))
                .into());
            }
            if !node_ids.contains(edge.target.as_str()) {
                return Err(ValidationError::state(format!(
                    "edge references non-existent target node: '{}'",
                    edge.target
                ))
                .into());
            }
        }
        Ok(())
    }

    fn validate_terminal_path(&self) -> Result<()> {
        if self.nodes.is_empty() {
            return Err(ValidationError::state("workflow has no nodes").into());
        }

        let has_terminal = self.nodes.iter().any(|node| {
            let id = node.id();
            let has_outgoing = self.edges.iter().any(|e| e.source == *id);
            !has_outgoing
        });

        if !has_terminal {
            return Err(ValidationError::state(
                "no terminal path found - every path must end in a node with no outgoing edges",
            )
            .into());
        }
        Ok(())
    }

    fn validate_join_reducer(&self) -> Result<()> {
        let node_ids: std::collections::HashSet<&str> = self.nodes.iter().map(|n| n.id()).collect();

        for edge in &self.edges {
            if matches!(edge.edge_type, EdgeType::Join) {
                if !node_ids.contains(edge.target.as_str()) {
                    return Err(ValidationError::state(format!(
                        "join edge targets non-existent node: '{}'",
                        edge.target
                    ))
                    .into());
                }
                let target_is_reducer = self.nodes.iter().any(|n| {
                    if let GraphNode::Reducer { id, .. } = n {
                        return id == edge.target.as_str();
                    }
                    false
                });
                if !target_is_reducer {
                    return Err(ValidationError::state(format!(
                        "join edge '{}' -> '{}' targets a node that is not a Reducer",
                        edge.source, edge.target
                    ))
                    .into());
                }
            }
        }
        Ok(())
    }

    fn validate_acyclicity(&self) -> Result<()> {
        if let Some(entry) = &self.entry_point {
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut in_progress: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            self.detect_cycle(entry, &mut visited, &mut in_progress)?;
        }
        Ok(())
    }

    fn detect_cycle(
        &self,
        node_id: &str,
        visited: &mut std::collections::HashSet<String>,
        in_progress: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if in_progress.contains(node_id) {
            return Err(
                ValidationError::state(format!("cycle detected at node: '{}'", node_id)).into(),
            );
        }
        if visited.contains(node_id) {
            return Ok(());
        }
        in_progress.insert(node_id.to_string());
        visited.insert(node_id.to_string());
        for edge in &self.edges {
            if edge.source == node_id {
                self.detect_cycle(&edge.target, visited, in_progress)?;
            }
        }
        in_progress.remove(node_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::DummyServices;
    use crate::errors::StepError;
    use crate::steps::{BoxedReducer, BoxedRouter, BoxedTransformer, BoxedValidator};
    use crate::steps::{Reducer, Router, Transformer, Validator};
    use naaf_schema::state::StateEnvelope;

    struct TestTransformer;
    impl Transformer for TestTransformer {
        type Services = DummyServices;
        fn name(&self) -> &'static str {
            "TestTransformer"
        }
        fn transform(
            &self,
            _: &mut crate::budget::ExecCtx<Self::Services>,
            input: StateEnvelope,
        ) -> std::result::Result<StateEnvelope, StepError> {
            Ok(input)
        }
    }

    struct TestRouter;
    impl Router for TestRouter {
        type Services = DummyServices;
        fn name(&self) -> &'static str {
            "TestRouter"
        }
        fn route(
            &self,
            _: &mut crate::budget::ExecCtx<Self::Services>,
            _: &StateEnvelope,
        ) -> std::result::Result<crate::route::RouteDecision, StepError> {
            Ok(crate::route::RouteDecision::Terminal)
        }
    }

    struct TestReducer;
    impl Reducer for TestReducer {
        type Services = DummyServices;
        fn name(&self) -> &'static str {
            "TestReducer"
        }
        fn reduce(
            &self,
            _: &mut crate::budget::ExecCtx<Self::Services>,
            inputs: Vec<StateEnvelope>,
        ) -> std::result::Result<StateEnvelope, StepError> {
            inputs
                .into_iter()
                .next()
                .ok_or_else(|| StepError::reducer("TestReducer", "empty inputs"))
        }
    }

    struct TestValidator;
    impl Validator for TestValidator {
        type Services = DummyServices;
        fn name(&self) -> &'static str {
            "TestValidator"
        }
        fn validate(
            &self,
            _: &crate::budget::ExecCtx<Self::Services>,
            _: &StateEnvelope,
        ) -> std::result::Result<(), crate::ValidationError> {
            Ok(())
        }
    }

    #[test]
    fn workflow_builder_new() {
        let builder = WorkflowBuilder::<DummyServices>::new("test");
        assert_eq!(builder.name, "test");
    }

    #[test]
    fn workflow_builder_step() {
        let workflow = WorkflowBuilder::<DummyServices>::new("test")
            .step("step1", BoxedTransformer::new(TestTransformer))
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile()
            .unwrap();
        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.entry_point, "step1");
        assert_eq!(workflow.nodes.len(), 2);
    }

    #[test]
    fn workflow_builder_multiple_steps() {
        let workflow = WorkflowBuilder::<DummyServices>::new("test")
            .step("step1", BoxedTransformer::new(TestTransformer))
            .step("step2", BoxedTransformer::new(TestTransformer))
            .path("step1", "step2")
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile()
            .unwrap();
        assert_eq!(workflow.nodes.len(), 3);
        assert_eq!(workflow.edges.len(), 1);
    }

    #[test]
    fn workflow_builder_route() {
        let workflow = WorkflowBuilder::<DummyServices>::new("test")
            .route("router1", BoxedRouter::new(TestRouter))
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile()
            .unwrap();
        assert_eq!(workflow.nodes.len(), 2);
    }

    #[test]
    fn workflow_builder_join() {
        let workflow = WorkflowBuilder::<DummyServices>::new("test")
            .join("join1", BoxedReducer::new(TestReducer))
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile()
            .unwrap();
        assert_eq!(workflow.nodes.len(), 2);
    }

    #[test]
    fn workflow_builder_terminal() {
        let workflow = WorkflowBuilder::<DummyServices>::new("test")
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile()
            .unwrap();
        assert_eq!(workflow.nodes.len(), 1);
    }

    #[test]
    fn workflow_builder_duplicate_step_id() {
        let result = WorkflowBuilder::<DummyServices>::new("test")
            .step("step1", BoxedTransformer::new(TestTransformer))
            .step("step1", BoxedTransformer::new(TestTransformer))
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile();
        assert!(result.is_err());
    }

    #[test]
    fn workflow_builder_cycle_detection() {
        let result = WorkflowBuilder::<DummyServices>::new("test")
            .step("step1", BoxedTransformer::new(TestTransformer))
            .step("step2", BoxedTransformer::new(TestTransformer))
            .path("step1", "step2")
            .path("step2", "step1")
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile();
        assert!(result.is_err());
    }

    #[test]
    fn workflow_builder_missing_reference() {
        let result = WorkflowBuilder::<DummyServices>::new("test")
            .step("step1", BoxedTransformer::new(TestTransformer))
            .path("step1", "nonexistent")
            .terminal("end", BoxedValidator::new(TestValidator))
            .compile();
        assert!(result.is_err());
    }

    #[test]
    fn workflow_builder_no_terminal() {
        let result = WorkflowBuilder::<DummyServices>::new("test")
            .step("step1", BoxedTransformer::new(TestTransformer))
            .step("step2", BoxedTransformer::new(TestTransformer))
            .path("step1", "step2")
            .path("step2", "step1") // Creates a cycle with no exit
            .compile();
        assert!(result.is_err());
    }
}
