use std::collections::HashMap;

use crate::budget::Services;
use crate::errors::{Error, Result};
use crate::graph::GraphNode;
use crate::steps::{BoxedReducer, BoxedRouter, BoxedTransformer, BoxedValidator};
use crate::workflow_package::{WorkflowNodeKind, WorkflowPackageNode};

type TransformerFactory<S> =
    Box<dyn Fn(&serde_json::Value) -> Result<BoxedTransformer<S>> + Send + Sync>;
type RouterFactory<S> = Box<dyn Fn(&serde_json::Value) -> Result<BoxedRouter<S>> + Send + Sync>;
type ReducerFactory<S> = Box<dyn Fn(&serde_json::Value) -> Result<BoxedReducer<S>> + Send + Sync>;
type ValidatorFactory<S> =
    Box<dyn Fn(&serde_json::Value) -> Result<BoxedValidator<S>> + Send + Sync>;

enum WorkflowStepFactory<S: Services> {
    Transformer(TransformerFactory<S>),
    Router(RouterFactory<S>),
    Reducer(ReducerFactory<S>),
    Validator(ValidatorFactory<S>),
}

pub struct WorkflowRegistry<S: Services> {
    factories: HashMap<String, WorkflowStepFactory<S>>,
}

impl<S: Services> Default for WorkflowRegistry<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Services> WorkflowRegistry<S> {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register_transformer<F>(&mut self, step: impl Into<String>, factory: F)
    where
        F: Fn(&serde_json::Value) -> Result<BoxedTransformer<S>> + Send + Sync + 'static,
    {
        self.factories.insert(
            step.into(),
            WorkflowStepFactory::Transformer(Box::new(factory)),
        );
    }

    pub fn register_router<F>(&mut self, step: impl Into<String>, factory: F)
    where
        F: Fn(&serde_json::Value) -> Result<BoxedRouter<S>> + Send + Sync + 'static,
    {
        self.factories
            .insert(step.into(), WorkflowStepFactory::Router(Box::new(factory)));
    }

    pub fn register_reducer<F>(&mut self, step: impl Into<String>, factory: F)
    where
        F: Fn(&serde_json::Value) -> Result<BoxedReducer<S>> + Send + Sync + 'static,
    {
        self.factories
            .insert(step.into(), WorkflowStepFactory::Reducer(Box::new(factory)));
    }

    pub fn register_validator<F>(&mut self, step: impl Into<String>, factory: F)
    where
        F: Fn(&serde_json::Value) -> Result<BoxedValidator<S>> + Send + Sync + 'static,
    {
        self.factories.insert(
            step.into(),
            WorkflowStepFactory::Validator(Box::new(factory)),
        );
    }

    pub fn build_node(&self, node: &WorkflowPackageNode) -> Result<GraphNode<S>> {
        let factory = self.factories.get(&node.step).ok_or_else(|| {
            Error::WorkflowPackage(format!(
                "Unknown workflow step '{}' referenced by node '{}'",
                node.step, node.id
            ))
        })?;

        match (node.kind, factory) {
            (WorkflowNodeKind::Transformer, WorkflowStepFactory::Transformer(factory)) => {
                Ok(GraphNode::transformer(&node.id, factory(&node.config)?))
            }
            (WorkflowNodeKind::Router, WorkflowStepFactory::Router(factory)) => {
                Ok(GraphNode::router(&node.id, factory(&node.config)?))
            }
            (WorkflowNodeKind::Reducer, WorkflowStepFactory::Reducer(factory)) => {
                Ok(GraphNode::reducer(&node.id, factory(&node.config)?))
            }
            (WorkflowNodeKind::Validator, WorkflowStepFactory::Validator(factory)) => {
                Ok(GraphNode::validator(&node.id, factory(&node.config)?))
            }
            _ => Err(Error::WorkflowPackage(format!(
                "Workflow node '{}' declares kind '{:?}' but step '{}' is registered differently",
                node.id, node.kind, node.step
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::DummyServices;
    use crate::errors::StepError;
    use crate::steps::{Transformer, Validator};

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

    struct TestValidator;

    impl Validator for TestValidator {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "test_validator"
        }

        fn validate(
            &self,
            _ctx: &crate::budget::ExecCtx<Self::Services>,
            _state: &naaf_schema::state::StateEnvelope,
        ) -> std::result::Result<(), crate::ValidationError> {
            Ok(())
        }
    }

    #[test]
    fn builds_registered_transformer_node() {
        let mut registry = WorkflowRegistry::<DummyServices>::new();
        registry.register_transformer("test.step", |_| Ok(BoxedTransformer::new(TestTransformer)));

        let node = WorkflowPackageNode {
            id: "start".to_string(),
            kind: WorkflowNodeKind::Transformer,
            step: "test.step".to_string(),
            config: serde_json::Value::Null,
        };

        let built = registry.build_node(&node).unwrap();
        assert_eq!(built.id(), "start");
    }

    #[test]
    fn rejects_kind_mismatch() {
        let mut registry = WorkflowRegistry::<DummyServices>::new();
        registry.register_validator("test.done", |_| Ok(BoxedValidator::new(TestValidator)));

        let node = WorkflowPackageNode {
            id: "start".to_string(),
            kind: WorkflowNodeKind::Transformer,
            step: "test.done".to_string(),
            config: serde_json::Value::Null,
        };

        let error = registry.build_node(&node).unwrap_err();
        assert!(error.to_string().contains("registered differently"));
    }
}
