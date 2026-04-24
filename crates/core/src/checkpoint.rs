use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{NodeId, NodeReport, RetryPolicy, StepReport, WorkflowRunId, graph::WorkflowNode};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Pluggable persistence backend for saving and loading workflow state.
///
/// All methods return pinned boxed futures so the trait is dyn-compatible
/// and can be stored as `Arc<dyn Checkpointer>`.
pub trait Checkpointer: Send + Sync + 'static {
    fn save_workflow(
        &self,
        run_id: WorkflowRunId,
        checkpoint: &WorkflowCheckpoint,
    ) -> BoxFuture<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>;

    fn load_workflow(
        &self,
        run_id: WorkflowRunId,
    ) -> BoxFuture<
        Result<Option<WorkflowCheckpoint>, Box<dyn std::error::Error + Send + Sync + 'static>>,
    >;

    fn save_step(
        &self,
        run_id: WorkflowRunId,
        node_id: NodeId,
        checkpoint: &StepCheckpoint,
    ) -> BoxFuture<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>;

    fn load_step(
        &self,
        run_id: WorkflowRunId,
        node_id: NodeId,
    ) -> BoxFuture<Result<Option<StepCheckpoint>, Box<dyn std::error::Error + Send + Sync + 'static>>>;

    fn delete_workflow(
        &self,
        run_id: WorkflowRunId,
    ) -> BoxFuture<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>;
}

pub trait StepCheckpointer: Send + Sync + 'static {
    fn checkpoint(&self, checkpoint: StepCheckpoint) -> BoxFuture<()>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeCheckpoint {
    pub id: NodeId,
    pub name: String,
    pub runner_key: Option<String>,
    pub seed: Option<Value>,
    pub parent: Option<NodeId>,
    pub dependencies: BTreeSet<NodeId>,
    pub downstream: BTreeSet<NodeId>,
    pub state: NodeCheckpointState,
    pub output: Option<Value>,
    pub report: NodeCheckpointReport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowCheckpoint {
    pub run_id: WorkflowRunId,
    pub max_concurrency: usize,
    pub nodes: BTreeMap<NodeId, NodeCheckpoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepCheckpoint {
    pub initial_input: Value,
    pub current_input: Value,
    pub repair_attempts: Vec<AttemptCheckpoint>,
    pub report_attempts: Vec<crate::repair::AttemptReport<Value>>,
    pub retry_policy: RetryPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCheckpointState {
    Pending,
    Running,
    Succeeded,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum NodeCheckpointReport {
    #[default]
    Empty,
    Step(StepReport<Value>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptCheckpoint {
    pub input: Value,
    pub output: Value,
    pub findings: Vec<Value>,
}

#[derive(Debug, Error)]
pub enum ResumeError {
    #[error("no runner registered for key '{key}'")]
    MissingRunner { key: String },
    #[error("checkpoint contains node '{node_id}' without runner key")]
    UnkeyedNode { node_id: NodeId },
    #[error("checkpointer error: {0}")]
    Checkpointer(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

pub struct RunnerRegistry<R, E> {
    runners: BTreeMap<String, Arc<dyn WorkflowNode<Runtime = R, Error = E>>>,
}

impl NodeCheckpointState {
    pub fn from_pending() -> Self {
        Self::Pending
    }

    pub fn from_running() -> Self {
        Self::Running
    }

    pub fn from_succeeded() -> Self {
        Self::Succeeded
    }
}

impl NodeCheckpointReport {
    pub fn from_node_report(report: &NodeReport) -> Self {
        match report {
            NodeReport::Empty => Self::Empty,
            NodeReport::Step(step_report) => Self::Step(step_report.clone()),
        }
    }
}

impl<R, E> Default for RunnerRegistry<R, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, E> RunnerRegistry<R, E> {
    pub fn new() -> Self {
        Self {
            runners: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        key: impl Into<String>,
        runner: Arc<dyn WorkflowNode<Runtime = R, Error = E>>,
    ) {
        self.runners.insert(key.into(), runner);
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn WorkflowNode<Runtime = R, Error = E>>> {
        self.runners.get(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.runners.keys().map(|k| k.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repair::AttemptReport;

    #[test]
    fn workflow_checkpoint_round_trips_through_json() {
        let run_id = WorkflowRunId::new();
        let node_id = NodeId::new();
        let checkpoint = WorkflowCheckpoint {
            run_id,
            max_concurrency: 4,
            nodes: {
                let mut nodes = BTreeMap::new();
                nodes.insert(
                    node_id,
                    NodeCheckpoint {
                        id: node_id,
                        name: "test_node".to_string(),
                        runner_key: Some("increment".to_string()),
                        seed: Some(serde_json::json!(42)),
                        parent: None,
                        dependencies: BTreeSet::new(),
                        downstream: BTreeSet::new(),
                        state: NodeCheckpointState::Succeeded,
                        output: Some(serde_json::json!(43)),
                        report: NodeCheckpointReport::Empty,
                    },
                );
                nodes
            },
        };

        let json = serde_json::to_string(&checkpoint).expect("should serialise");
        let decoded: WorkflowCheckpoint = serde_json::from_str(&json).expect("should deserialise");
        assert_eq!(decoded.run_id, run_id);
        assert_eq!(decoded.max_concurrency, 4);
        assert_eq!(decoded.nodes.len(), 1);
        assert_eq!(decoded.nodes[&node_id].name, "test_node");
        assert_eq!(
            decoded.nodes[&node_id].runner_key,
            Some("increment".to_string())
        );
    }

    #[test]
    fn step_checkpoint_round_trips_through_json() {
        let checkpoint = StepCheckpoint {
            initial_input: serde_json::json!(0),
            current_input: serde_json::json!(2),
            repair_attempts: vec![AttemptCheckpoint {
                input: serde_json::json!(0),
                output: serde_json::json!(1),
                findings: vec![serde_json::json!("too low")],
            }],
            report_attempts: vec![AttemptReport {
                findings: vec![serde_json::json!("too low")],
                accepted: false,
            }],
            retry_policy: RetryPolicy::new(3),
        };

        let json = serde_json::to_string(&checkpoint).expect("should serialise");
        let decoded: StepCheckpoint = serde_json::from_str(&json).expect("should deserialise");
        assert_eq!(decoded.current_input, serde_json::json!(2));
        assert_eq!(decoded.repair_attempts.len(), 1);
        assert_eq!(decoded.retry_policy.max_attempts(), 3);
    }

    #[test]
    fn node_checkpoint_report_from_node_report() {
        let step_report: StepReport<Value> = StepReport::new(vec![AttemptReport {
            findings: vec![serde_json::json!("failure")],
            accepted: false,
        }]);
        let report = NodeReport::Step(step_report);
        let cp_report = NodeCheckpointReport::from_node_report(&report);
        assert!(matches!(cp_report, NodeCheckpointReport::Step(_)));

        let cp_report = NodeCheckpointReport::from_node_report(&NodeReport::Empty);
        assert!(matches!(cp_report, NodeCheckpointReport::Empty));
    }
}
