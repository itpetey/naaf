use std::fmt;

use crate::budget::Services;
use crate::errors::ValidationError;
use crate::steps::{BoxedReducer, BoxedRouter, BoxedTransformer, BoxedValidator};

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeType {
    Normal,
    Conditional,
    Join,
}

#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
}

impl GraphEdge {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            edge_type: EdgeType::Normal,
        }
    }

    pub fn conditional(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            edge_type: EdgeType::Conditional,
        }
    }

    pub fn join(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            edge_type: EdgeType::Join,
        }
    }
}

pub enum GraphNode<S: Services> {
    Transformer {
        id: String,
        transformer: BoxedTransformer<S>,
    },
    Router {
        id: String,
        router: BoxedRouter<S>,
    },
    Reducer {
        id: String,
        reducer: BoxedReducer<S>,
    },
    Validator {
        id: String,
        validator: BoxedValidator<S>,
    },
}

impl<S: Services> GraphNode<S> {
    pub fn id(&self) -> &str {
        match self {
            Self::Transformer { id, .. } => id,
            Self::Router { id, .. } => id,
            Self::Reducer { id, .. } => id,
            Self::Validator { id, .. } => id,
        }
    }

    pub fn transformer(id: impl Into<String>, transformer: BoxedTransformer<S>) -> Self {
        Self::Transformer {
            id: id.into(),
            transformer,
        }
    }

    pub fn router(id: impl Into<String>, router: BoxedRouter<S>) -> Self {
        Self::Router {
            id: id.into(),
            router,
        }
    }

    pub fn reducer(id: impl Into<String>, reducer: BoxedReducer<S>) -> Self {
        Self::Reducer {
            id: id.into(),
            reducer,
        }
    }

    pub fn validator(id: impl Into<String>, validator: BoxedValidator<S>) -> Self {
        Self::Validator {
            id: id.into(),
            validator,
        }
    }
}

impl<S: Services> fmt::Debug for GraphNode<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transformer { id, .. } => f.debug_struct("Transformer").field("id", id).finish(),
            Self::Router { id, .. } => f.debug_struct("Router").field("id", id).finish(),
            Self::Reducer { id, .. } => f.debug_struct("Reducer").field("id", id).finish(),
            Self::Validator { id, .. } => f.debug_struct("Validator").field("id", id).finish(),
        }
    }
}

pub struct CompiledWorkflow<S: Services> {
    pub name: String,
    pub nodes: Vec<GraphNode<S>>,
    pub edges: Vec<GraphEdge>,
    pub entry_point: String,
}

impl<S: Services> CompiledWorkflow<S> {
    pub fn new(name: impl Into<String>, entry_point: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            entry_point: entry_point.into(),
        }
    }

    pub fn add_node(&mut self, node: GraphNode<S>) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&GraphNode<S>> {
        self.nodes.iter().find(|n| n.id() == id)
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.nodes.is_empty() {
            return Err(ValidationError::state("workflow has no nodes"));
        }

        if self.get_node(&self.entry_point).is_none() {
            return Err(ValidationError::state(format!(
                "entry point '{}' not found in nodes",
                self.entry_point
            )));
        }

        Ok(())
    }

    pub fn topological_sort(&self) -> Result<Vec<&GraphNode<S>>, ValidationError> {
        let mut in_degree: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for node in &self.nodes {
            in_degree.insert(node.id(), 0);
        }
        for edge in &self.edges {
            if let Some(degree) = in_degree.get_mut(&edge.target.as_str()) {
                *degree += 1;
            }
        }

        let mut queue: Vec<_> = in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut sorted = Vec::new();
        while let Some(node_id) = queue.pop() {
            if let Some(node) = self.nodes.iter().find(|n| n.id() == node_id) {
                sorted.push(node);
            }
            for edge in &self.edges {
                if edge.source == node_id
                    && let Some(degree) = in_degree.get_mut(&edge.target.as_str())
                {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(edge.target.as_str());
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            return Err(ValidationError::state(
                "graph contains cycles - topological sort failed",
            ));
        }

        Ok(sorted)
    }
}

impl<S: Services> fmt::Debug for CompiledWorkflow<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledWorkflow")
            .field("name", &self.name)
            .field(
                "nodes",
                &self.nodes.iter().map(|n| n.id()).collect::<Vec<_>>(),
            )
            .field("edges", &self.edges)
            .field("entry_point", &self.entry_point)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::DummyServices;
    use crate::errors::StepError;
    use crate::steps::{BoxedReducer, BoxedRouter, BoxedTransformer};
    use crate::steps::{Reducer, Router, Transformer, Validator};
    use workflow_schema::state::StateEnvelope;

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
    fn compiled_workflow_new() {
        let workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "start");
        assert_eq!(workflow.name, "test");
        assert_eq!(workflow.entry_point, "start");
        assert!(workflow.nodes.is_empty());
        assert!(workflow.edges.is_empty());
    }

    #[test]
    fn compiled_workflow_add_node() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "start");
        let node = GraphNode::transformer("step1", BoxedTransformer::new(TestTransformer));
        workflow.add_node(node);
        assert_eq!(workflow.nodes.len(), 1);
        assert_eq!(workflow.nodes[0].id(), "step1");
    }

    #[test]
    fn compiled_workflow_add_edge() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "start");
        workflow.add_edge(GraphEdge::new("a", "b"));
        assert_eq!(workflow.edges.len(), 1);
    }

    #[test]
    fn compiled_workflow_get_node() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "start");
        let node = GraphNode::transformer("step1", BoxedTransformer::new(TestTransformer));
        workflow.add_node(node);
        assert!(workflow.get_node("step1").is_some());
        assert!(workflow.get_node("nonexistent").is_none());
    }

    #[test]
    fn compiled_workflow_validate_empty() {
        let workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "start");
        let result = workflow.validate();
        assert!(result.is_err());
    }

    #[test]
    fn compiled_workflow_validate_invalid_entry() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "start");
        let node = GraphNode::transformer("step1", BoxedTransformer::new(TestTransformer));
        workflow.add_node(node);
        let result = workflow.validate();
        assert!(result.is_err());
    }

    #[test]
    fn compiled_workflow_validate_valid() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "step1");
        let node = GraphNode::transformer("step1", BoxedTransformer::new(TestTransformer));
        workflow.add_node(node);
        let result = workflow.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn graph_edge_types() {
        let normal = GraphEdge::new("a", "b");
        assert_eq!(normal.edge_type, EdgeType::Normal);

        let conditional = GraphEdge::conditional("a", "b");
        assert_eq!(conditional.edge_type, EdgeType::Conditional);

        let join = GraphEdge::join("a", "b");
        assert_eq!(join.edge_type, EdgeType::Join);
    }

    #[test]
    fn topological_sort_linear() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "a");
        workflow.add_node(GraphNode::transformer(
            "a",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "b",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "c",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_edge(GraphEdge::new("a", "b"));
        workflow.add_edge(GraphEdge::new("b", "c"));

        let sorted = workflow.topological_sort().unwrap();
        let ids: Vec<_> = sorted.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn topological_sort_parallel() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "a");
        workflow.add_node(GraphNode::transformer(
            "a",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "b",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "c",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_edge(GraphEdge::new("a", "b"));
        workflow.add_edge(GraphEdge::new("a", "c"));

        let sorted = workflow.topological_sort().unwrap();
        let ids: Vec<_> = sorted.iter().map(|n| n.id()).collect();
        assert_eq!(ids[0], "a");
        assert!(ids[1..].contains(&"b") && ids[1..].contains(&"c"));
    }

    #[test]
    fn topological_sort_cycle_fails() {
        let mut workflow: CompiledWorkflow<DummyServices> = CompiledWorkflow::new("test", "a");
        workflow.add_node(GraphNode::transformer(
            "a",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_node(GraphNode::transformer(
            "b",
            BoxedTransformer::new(TestTransformer),
        ));
        workflow.add_edge(GraphEdge::new("a", "b"));
        workflow.add_edge(GraphEdge::new("b", "a"));

        let result = workflow.topological_sort();
        assert!(result.is_err());
    }
}
