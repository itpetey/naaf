use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{Display, Formatter},
    sync::Arc,
};

use futures::{
    future::LocalBoxFuture,
    stream::{FuturesUnordered, StreamExt},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

use crate::{
    AttemptReport, Step, StepError, StepReport, SystemStage,
    checkpoint::{
        Checkpointer, NodeCheckpoint, NodeCheckpointReport, NodeCheckpointState, ResumeError,
        RunnerRegistry, WorkflowCheckpoint,
    },
    repair::NeverFinding,
};

type BuildPatch<R, O, E> = dyn Fn(&NodeContext, &O) -> GraphPatch<R, E>;
type InFlightNode<'a, R, E> = LocalBoxFuture<'a, (NodeId, NodeResult<R, E>)>;
type NodeFuture<'a, R, E> = LocalBoxFuture<'a, NodeResult<R, E>>;
type NodeResult<R, E> = Result<NodeOutcome<R, E>, NodeExecutionError<E>>;
type NodeRunner<R, E> = dyn WorkflowNode<Runtime = R, Error = E>;

/// A runnable node in the dynamic workflow graph.
pub trait WorkflowNode {
    /// Shared runtime capabilities used by this node.
    type Runtime;
    /// Errors returned by the wrapped task logic.
    type Error;

    /// Executes the node and optionally returns new downstream work.
    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        context: NodeContext,
        input: NodeInput,
    ) -> NodeFuture<'a, Self::Runtime, Self::Error>;
}

/// Final graph state after a successful workflow run.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowRunReport {
    run_id: WorkflowRunId,
    nodes: BTreeMap<NodeId, NodeSummary>,
}

/// Immutable execution metadata passed into a running workflow node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeContext {
    run_id: WorkflowRunId,
    node_id: NodeId,
    parent_id: Option<NodeId>,
}

/// Summary for one node after a successful workflow run.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSummary {
    id: NodeId,
    name: String,
    parent_id: Option<NodeId>,
    output: Value,
    report: NodeReport,
}

/// Input visible to one dynamic workflow node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeInput {
    seed: Option<Value>,
    upstream: BTreeMap<NodeId, Value>,
}

/// Input-selection failures while adapting dynamic node inputs into typed values.
#[derive(Debug, Error)]
pub enum InputSelectionError {
    /// A seed input was required but this node had none.
    #[error("seed input was required but missing")]
    MissingSeedInput,
    /// A required upstream output was missing.
    #[error("upstream output for node '{upstream}' was required but missing")]
    MissingUpstreamOutput { upstream: NodeId },
    /// The selected JSON value could not be decoded into the expected type.
    #[error("failed to decode node input: {0}")]
    DecodeInput(#[source] serde_json::Error),
}

/// Invalid additive graph mutations rejected by the scheduler.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum InvalidPatchError {
    /// The patch introduced a node identifier that already existed.
    #[error("node '{node_id}' already exists in the workflow")]
    DuplicateNodeId { node_id: NodeId },
    /// The patch repeated the same edge twice.
    #[error("edge '{from}' -> '{to}' is duplicated in the patch")]
    DuplicateEdge { from: NodeId, to: NodeId },
    /// The patch referenced a source node that does not exist.
    #[error("edge source '{node_id}' does not exist")]
    UnknownEdgeSource { node_id: NodeId },
    /// The patch referenced a target node that does not exist.
    #[error("edge target '{node_id}' does not exist")]
    UnknownEdgeTarget { node_id: NodeId },
    /// Additive patches may only target nodes created by the patch itself.
    #[error("edge target '{node_id}' must be newly added by the patch")]
    ExistingTarget { node_id: NodeId },
    /// The patch would make the graph cyclic.
    #[error("patch would introduce a cycle in the workflow graph")]
    CycleDetected,
}

/// One additive edge insertion from an upstream node into a new downstream node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSpec {
    from: NodeId,
    to: NodeId,
}

struct NodeRecord<R, E> {
    id: NodeId,
    name: String,
    runner_key: Option<String>,
    seed: Option<Value>,
    parent: Option<NodeId>,
    dependencies: BTreeSet<NodeId>,
    downstream: BTreeSet<NodeId>,
    state: NodeState,
    output: Option<Value>,
    report: NodeReport,
    runner: Arc<NodeRunner<R, E>>,
}

/// Unique identifier for one workflow run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkflowRunId(Uuid);

/// Unique identifier for one node within a workflow graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

/// Dynamic node execution errors.
#[derive(Debug, Error)]
pub enum NodeExecutionError<E> {
    /// The node returned a domain or infrastructure error.
    #[error("node execution failed: {0}")]
    System(#[source] E),
    /// A wrapped step failed during one of its system stages.
    #[error("{stage} failed: {error}")]
    StepSystem {
        stage: SystemStage,
        #[source]
        error: E,
    },
    /// A wrapped step exhausted retries and was rejected.
    #[error("step rejected after {attempts} attempt(s)")]
    StepRejected {
        report: StepReport<Value>,
        attempts: usize,
    },
    /// The selected JSON input could not be decoded.
    #[error(transparent)]
    InputSelection(#[from] InputSelectionError),
    /// The node output could not be serialised.
    #[error("failed to encode node output: {0}")]
    EncodeOutput(#[source] serde_json::Error),
    /// Step findings could not be serialised into the dynamic report.
    #[error("failed to encode step findings: {0}")]
    EncodeFindings(#[source] serde_json::Error),
}

/// Workflow graph validation and execution errors.
#[derive(Debug, Error)]
pub enum WorkflowError<E> {
    /// The graph patch attempted an unsupported or invalid mutation.
    #[error(transparent)]
    InvalidPatch(#[from] InvalidPatchError),
    /// One node failed while the graph was running.
    #[error("node '{node_id}' failed: {error}")]
    Node {
        node_id: NodeId,
        #[source]
        error: NodeExecutionError<E>,
    },
    /// No runnable nodes were left even though the workflow was incomplete.
    #[error("workflow stalled with pending nodes: {pending:?}")]
    Stalled { pending: Vec<NodeId> },
}

/// Per-node execution metadata returned after a successful workflow run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum NodeReport {
    /// The node did not expose a structured execution report.
    #[default]
    Empty,
    /// The node wrapped a `Step` and reports its attempt history.
    Step(StepReport<Value>),
}

/// The completed result of a dynamic node execution.
pub struct NodeOutcome<R, E> {
    output: Value,
    patch: GraphPatch<R, E>,
    report: NodeReport,
}

/// A planned node insertion accepted by the graph scheduler.
pub struct NodeSpec<R, E> {
    id: NodeId,
    name: String,
    runner_key: Option<String>,
    seed: Option<Value>,
    parent: Option<NodeId>,
    runner: Arc<NodeRunner<R, E>>,
}

/// An additive mutation applied to the running workflow graph.
pub struct GraphPatch<R, E> {
    nodes: Vec<NodeSpec<R, E>>,
    edges: Vec<EdgeSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum NodeState {
    Pending,
    Running,
    Succeeded,
}

/// Runs a mutable workflow graph with additive downstream node insertion.
pub struct Workflow<R, E> {
    run_id: WorkflowRunId,
    max_concurrency: usize,
    checkpointer: Option<Arc<dyn Checkpointer>>,
    registry: Option<RunnerRegistry<R, E>>,
    nodes: BTreeMap<NodeId, NodeRecord<R, E>>,
}

/// Adapts a typed `Step` into a dynamic workflow node with optional downstream spawning.
pub struct StepNode<R, I, O, F, E, Select> {
    step: Step<R, I, O, F, E>,
    select_input: Select,
    build_patch: Option<Arc<BuildPatch<R, O, E>>>,
}

impl EdgeSpec {
    /// Creates a directed dependency edge.
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self { from, to }
    }

    /// Returns the edge source node.
    pub fn from(&self) -> NodeId {
        self.from
    }

    /// Returns the edge target node.
    pub fn to(&self) -> NodeId {
        self.to
    }
}

impl WorkflowRunId {
    /// Creates a fresh workflow run identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WorkflowRunId {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeId {
    /// Creates a fresh node identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for WorkflowRunId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl NodeContext {
    /// Creates a node context for the given run, node, and optional parent.
    pub fn new(run_id: WorkflowRunId, node_id: NodeId, parent_id: Option<NodeId>) -> Self {
        Self {
            run_id,
            node_id,
            parent_id,
        }
    }

    /// Returns the current workflow run identifier.
    pub fn run_id(&self) -> WorkflowRunId {
        self.run_id
    }

    /// Returns the currently executing node identifier.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Returns the node that created this node, if any.
    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent_id
    }
}

impl NodeInput {
    /// Creates a node input from an optional seed and upstream outputs.
    pub fn new(seed: Option<Value>, upstream: BTreeMap<NodeId, Value>) -> Self {
        Self { seed, upstream }
    }

    /// Returns the seed value configured for this node, if any.
    pub fn seed(&self) -> Option<&Value> {
        self.seed.as_ref()
    }

    /// Decodes the seed value into a typed input.
    pub fn seed_as<T>(&self) -> Result<T, InputSelectionError>
    where
        T: DeserializeOwned,
    {
        let seed = self
            .seed
            .clone()
            .ok_or(InputSelectionError::MissingSeedInput)?;
        serde_json::from_value(seed).map_err(InputSelectionError::DecodeInput)
    }

    /// Returns every upstream output keyed by node identifier.
    pub fn upstream(&self) -> &BTreeMap<NodeId, Value> {
        &self.upstream
    }

    /// Returns the output from one upstream node, if present.
    pub fn output(&self, node_id: NodeId) -> Option<&Value> {
        self.upstream.get(&node_id)
    }

    /// Decodes one upstream output into a typed value.
    pub fn output_as<T>(&self, node_id: NodeId) -> Result<T, InputSelectionError>
    where
        T: DeserializeOwned,
    {
        let output = self
            .upstream
            .get(&node_id)
            .cloned()
            .ok_or(InputSelectionError::MissingUpstreamOutput { upstream: node_id })?;
        serde_json::from_value(output).map_err(InputSelectionError::DecodeInput)
    }
}

impl<R, E> NodeOutcome<R, E> {
    /// Creates a completed node outcome with no spawned downstream work.
    pub fn new(output: Value) -> Self {
        Self {
            output,
            patch: GraphPatch::new(),
            report: NodeReport::Empty,
        }
    }

    /// Attaches an additive downstream graph patch.
    pub fn with_patch(mut self, patch: GraphPatch<R, E>) -> Self {
        self.patch = patch;
        self
    }

    /// Attaches a structured execution report.
    pub fn with_report(mut self, report: NodeReport) -> Self {
        self.report = report;
        self
    }
}

impl<R, E> Clone for NodeSpec<R, E> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            runner_key: self.runner_key.clone(),
            seed: self.seed.clone(),
            parent: self.parent,
            runner: self.runner.clone(),
        }
    }
}

impl<R, E> NodeSpec<R, E> {
    /// Creates a new node specification around the given runner.
    pub fn new(
        name: impl Into<String>,
        runner: impl WorkflowNode<Runtime = R, Error = E> + 'static,
    ) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            runner_key: None,
            seed: None,
            parent: None,
            runner: Arc::new(runner),
        }
    }

    /// Creates a node specification from a pre-shared runner.
    pub fn from_shared_runner(name: impl Into<String>, runner: Arc<NodeRunner<R, E>>) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            runner_key: None,
            seed: None,
            parent: None,
            runner,
        }
    }

    /// Overrides the generated node identifier.
    pub fn with_id(mut self, id: NodeId) -> Self {
        self.id = id;
        self
    }

    /// Configures the seed input for this node.
    pub fn with_seed<T>(mut self, seed: T) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        self.seed = Some(serde_json::to_value(seed)?);
        Ok(self)
    }

    /// Configures the seed input as a prebuilt JSON value.
    pub fn with_seed_value(mut self, seed: Value) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Records which node created this node.
    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Sets the runner registry key for checkpoint/resume support.
    pub fn with_runner_key(mut self, key: impl Into<String>) -> Self {
        self.runner_key = Some(key.into());
        self
    }

    /// Returns the node identifier.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns the configured public node name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<R, E> Default for GraphPatch<R, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, E> GraphPatch<R, E> {
    /// Creates an empty graph patch.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Returns the nodes introduced by this patch.
    pub fn nodes(&self) -> &[NodeSpec<R, E>] {
        &self.nodes
    }

    /// Returns the edges introduced by this patch.
    pub fn edges(&self) -> &[EdgeSpec] {
        &self.edges
    }

    /// Adds one new node to the patch.
    pub fn with_node(mut self, node: NodeSpec<R, E>) -> Self {
        self.nodes.push(node);
        self
    }

    /// Adds one new edge to the patch.
    pub fn with_edge(mut self, edge: EdgeSpec) -> Self {
        self.edges.push(edge);
        self
    }

    fn into_parts(self) -> (Vec<NodeSpec<R, E>>, Vec<EdgeSpec>) {
        (self.nodes, self.edges)
    }
}

impl NodeSummary {
    /// Returns the node identifier.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns the public node name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the parent node that created this node, if any.
    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent_id
    }

    /// Returns the successful node output.
    pub fn output(&self) -> &Value {
        &self.output
    }

    /// Returns the node execution report.
    pub fn report(&self) -> &NodeReport {
        &self.report
    }
}

impl WorkflowRunReport {
    /// Returns the workflow run identifier.
    pub fn run_id(&self) -> WorkflowRunId {
        self.run_id
    }

    /// Returns every completed node summary.
    pub fn nodes(&self) -> &BTreeMap<NodeId, NodeSummary> {
        &self.nodes
    }

    /// Returns one node summary by identifier.
    pub fn node(&self, node_id: NodeId) -> Option<&NodeSummary> {
        self.nodes.get(&node_id)
    }
}

impl<R, E> Default for Workflow<R, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, E> Workflow<R, E> {
    /// Creates an empty workflow graph.
    pub fn new() -> Self {
        Self {
            run_id: WorkflowRunId::new(),
            max_concurrency: usize::MAX,
            checkpointer: None,
            registry: None,
            nodes: BTreeMap::new(),
        }
    }

    /// Sets the maximum number of nodes executed concurrently.
    pub fn with_max_concurrency(mut self, max_concurrency: usize) -> Self {
        assert!(
            max_concurrency > 0,
            "workflow must allow at least one concurrent node"
        );
        self.max_concurrency = max_concurrency;
        self
    }

    /// Installs a checkpointer for saving workflow state after each node completion.
    pub fn with_checkpointer(mut self, checkpointer: impl Checkpointer + 'static) -> Self {
        self.checkpointer = Some(Arc::new(checkpointer));
        self
    }

    /// Installs a runner registry for mapping names to node runners during resume.
    pub fn with_registry(mut self, registry: RunnerRegistry<R, E>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Reconstructs a workflow from a previously saved checkpoint.
    ///
    /// Nodes that were running when the checkpoint was captured are reset to
    /// pending so they will be re-executed. Succeeded nodes retain their
    /// outputs and will not be re-run.
    pub fn resume(
        checkpoint: WorkflowCheckpoint,
        registry: RunnerRegistry<R, E>,
    ) -> Result<Self, ResumeError> {
        let mut workflow = Self {
            run_id: checkpoint.run_id,
            max_concurrency: checkpoint.max_concurrency,
            checkpointer: None,
            registry: Some(registry),
            nodes: BTreeMap::new(),
        };

        let registry_ref = workflow.registry.as_ref().expect("registry just set");

        for (node_id, node_cp) in checkpoint.nodes {
            let runner_key = node_cp.runner_key.as_ref();
            let runner = runner_key
                .ok_or(ResumeError::UnkeyedNode { node_id })
                .and_then(|key| {
                    registry_ref
                        .get(key)
                        .cloned()
                        .ok_or_else(|| ResumeError::MissingRunner { key: key.clone() })
                })?;

            let state = match node_cp.state {
                NodeCheckpointState::Pending => NodeState::Pending,
                NodeCheckpointState::Running => NodeState::Pending,
                NodeCheckpointState::Succeeded => NodeState::Succeeded,
            };

            workflow.nodes.insert(
                node_id,
                NodeRecord {
                    id: node_id,
                    name: node_cp.name,
                    runner_key: node_cp.runner_key,
                    seed: node_cp.seed,
                    parent: node_cp.parent,
                    dependencies: node_cp.dependencies,
                    downstream: node_cp.downstream,
                    state,
                    output: node_cp.output,
                    report: match node_cp.report {
                        NodeCheckpointReport::Empty => NodeReport::Empty,
                        NodeCheckpointReport::Step(report) => NodeReport::Step(report),
                    },
                    runner,
                },
            );
        }

        Ok(workflow)
    }

    /// Applies one additive patch before or during execution.
    pub fn apply_patch(&mut self, patch: GraphPatch<R, E>) -> Result<(), InvalidPatchError> {
        let (nodes, edges) = patch.into_parts();
        self.validate_patch(&nodes, &edges)?;

        for node in nodes {
            let id = node.id;
            self.nodes.insert(
                id,
                NodeRecord {
                    id,
                    name: node.name,
                    runner_key: node.runner_key,
                    seed: node.seed,
                    parent: node.parent,
                    dependencies: BTreeSet::new(),
                    downstream: BTreeSet::new(),
                    state: NodeState::Pending,
                    output: None,
                    report: NodeReport::Empty,
                    runner: node.runner,
                },
            );
        }

        for edge in edges {
            if let Some(record) = self.nodes.get_mut(&edge.from) {
                record.downstream.insert(edge.to);
            }
            if let Some(record) = self.nodes.get_mut(&edge.to) {
                record.dependencies.insert(edge.from);
            }
        }

        Ok(())
    }

    /// Applies one patch and returns the workflow for chaining.
    pub fn with_patch(mut self, patch: GraphPatch<R, E>) -> Result<Self, InvalidPatchError> {
        self.apply_patch(patch)?;
        Ok(self)
    }

    /// Executes the workflow until every node succeeds or one node fails.
    pub fn run<'a>(
        mut self,
        runtime: &'a R,
    ) -> LocalBoxFuture<'a, Result<WorkflowRunReport, WorkflowError<E>>>
    where
        R: 'static,
        E: 'static,
    {
        Box::pin(async move {
            let mut in_flight: FuturesUnordered<InFlightNode<'a, R, E>> = FuturesUnordered::new();

            loop {
                while in_flight.len() < self.max_concurrency {
                    let Some(node_id) = self.next_ready_node() else {
                        break;
                    };

                    let Some(input) = self.build_input(node_id) else {
                        return Err(WorkflowError::Stalled {
                            pending: self.pending_nodes(),
                        });
                    };
                    let context = self.node_context(node_id);
                    let Some(runner) = self.nodes.get(&node_id).map(|record| record.runner.clone())
                    else {
                        return Err(WorkflowError::Stalled {
                            pending: self.pending_nodes(),
                        });
                    };

                    if let Some(record) = self.nodes.get_mut(&node_id) {
                        record.state = NodeState::Running;
                    }

                    in_flight.push(Self::run_node(runtime, node_id, runner, context, input));
                }

                if in_flight.is_empty() {
                    if self.pending_nodes().is_empty() {
                        return Ok(self.into_report());
                    }

                    return Err(WorkflowError::Stalled {
                        pending: self.pending_nodes(),
                    });
                }

                let Some((node_id, result)) = in_flight.next().await else {
                    continue;
                };

                let outcome = result.map_err(|error| WorkflowError::Node { node_id, error })?;
                let NodeOutcome {
                    output,
                    patch,
                    report,
                } = outcome;

                if let Some(record) = self.nodes.get_mut(&node_id) {
                    record.state = NodeState::Succeeded;
                    record.output = Some(output);
                    record.report = report;
                }

                self.apply_patch(patch)?;

                if let Some(checkpointer) = &self.checkpointer {
                    let checkpoint = self.build_checkpoint();
                    if let Err(error) = checkpointer.save_workflow(self.run_id, &checkpoint).await {
                        warn!(run_id = %self.run_id, %error, "failed to save workflow checkpoint");
                    }
                }
            }
        })
    }

    fn build_checkpoint(&self) -> WorkflowCheckpoint {
        let nodes = self
            .nodes
            .iter()
            .map(|(node_id, record)| {
                (
                    *node_id,
                    NodeCheckpoint {
                        id: record.id,
                        name: record.name.clone(),
                        runner_key: record.runner_key.clone(),
                        seed: record.seed.clone(),
                        parent: record.parent,
                        dependencies: record.dependencies.clone(),
                        downstream: record.downstream.clone(),
                        state: match record.state {
                            NodeState::Pending => NodeCheckpointState::Pending,
                            NodeState::Running => NodeCheckpointState::Running,
                            NodeState::Succeeded => NodeCheckpointState::Succeeded,
                        },
                        output: record.output.clone(),
                        report: NodeCheckpointReport::from_node_report(&record.report),
                    },
                )
            })
            .collect();

        WorkflowCheckpoint {
            run_id: self.run_id,
            max_concurrency: self.max_concurrency,
            nodes,
        }
    }

    fn next_ready_node(&self) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|(_, record)| {
                record.state == NodeState::Pending
                    && record.dependencies.iter().all(|dependency| {
                        self.nodes
                            .get(dependency)
                            .is_some_and(|node| node.state == NodeState::Succeeded)
                    })
            })
            .map(|(node_id, _)| *node_id)
    }

    fn pending_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, record)| record.state == NodeState::Pending)
            .map(|(node_id, _)| *node_id)
            .collect()
    }

    fn node_context(&self, node_id: NodeId) -> NodeContext {
        let parent_id = self.nodes.get(&node_id).and_then(|record| record.parent);
        NodeContext {
            run_id: self.run_id,
            node_id,
            parent_id,
        }
    }

    fn build_input(&self, node_id: NodeId) -> Option<NodeInput> {
        let record = self.nodes.get(&node_id)?;
        let upstream = record
            .dependencies
            .iter()
            .map(|dependency| {
                self.nodes
                    .get(dependency)
                    .and_then(|node| node.output.clone())
                    .map(|output| (*dependency, output))
            })
            .collect::<Option<BTreeMap<_, _>>>()?;

        Some(NodeInput::new(record.seed.clone(), upstream))
    }

    fn run_node<'a>(
        runtime: &'a R,
        node_id: NodeId,
        runner: Arc<NodeRunner<R, E>>,
        context: NodeContext,
        input: NodeInput,
    ) -> InFlightNode<'a, R, E>
    where
        R: 'static,
        E: 'static,
    {
        Box::pin(async move { (node_id, runner.run(runtime, context, input).await) })
    }

    fn validate_patch(
        &self,
        nodes: &[NodeSpec<R, E>],
        edges: &[EdgeSpec],
    ) -> Result<(), InvalidPatchError> {
        let mut new_ids = BTreeSet::new();
        for node in nodes {
            if self.nodes.contains_key(&node.id) || !new_ids.insert(node.id) {
                return Err(InvalidPatchError::DuplicateNodeId { node_id: node.id });
            }
        }

        let mut edge_set = BTreeSet::new();
        let known_ids = self
            .nodes
            .keys()
            .copied()
            .chain(new_ids.iter().copied())
            .collect::<BTreeSet<_>>();

        for edge in edges {
            if !edge_set.insert((edge.from, edge.to)) {
                return Err(InvalidPatchError::DuplicateEdge {
                    from: edge.from,
                    to: edge.to,
                });
            }

            if !known_ids.contains(&edge.from) {
                return Err(InvalidPatchError::UnknownEdgeSource { node_id: edge.from });
            }

            if !known_ids.contains(&edge.to) {
                return Err(InvalidPatchError::UnknownEdgeTarget { node_id: edge.to });
            }

            if !new_ids.contains(&edge.to) {
                return Err(InvalidPatchError::ExistingTarget { node_id: edge.to });
            }
        }

        let mut indegree = self
            .nodes
            .keys()
            .copied()
            .map(|node_id| (node_id, 0usize))
            .chain(new_ids.iter().copied().map(|node_id| (node_id, 0usize)))
            .collect::<BTreeMap<_, _>>();
        let mut adjacency = self
            .nodes
            .keys()
            .copied()
            .map(|node_id| (node_id, BTreeSet::new()))
            .chain(
                new_ids
                    .iter()
                    .copied()
                    .map(|node_id| (node_id, BTreeSet::new())),
            )
            .collect::<BTreeMap<_, _>>();

        for record in self.nodes.values() {
            for downstream in &record.downstream {
                if let Some(targets) = adjacency.get_mut(&record.id) {
                    targets.insert(*downstream);
                }
                if let Some(degree) = indegree.get_mut(downstream) {
                    *degree += 1;
                }
            }
        }

        for edge in edges {
            if let Some(targets) = adjacency.get_mut(&edge.from) {
                targets.insert(edge.to);
            }
            if let Some(degree) = indegree.get_mut(&edge.to) {
                *degree += 1;
            }
        }

        let mut ready = indegree
            .iter()
            .filter_map(|(node_id, degree)| (*degree == 0).then_some(*node_id))
            .collect::<Vec<_>>();
        let mut visited = 0usize;

        while let Some(node_id) = ready.pop() {
            visited += 1;
            let Some(downstream) = adjacency.get(&node_id) else {
                continue;
            };

            for next in downstream {
                if let Some(degree) = indegree.get_mut(next) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push(*next);
                    }
                }
            }
        }

        if visited != indegree.len() {
            return Err(InvalidPatchError::CycleDetected);
        }

        Ok(())
    }

    fn into_report(self) -> WorkflowRunReport {
        let nodes = self
            .nodes
            .into_iter()
            .filter_map(|(node_id, record)| {
                record.output.map(|output| {
                    (
                        node_id,
                        NodeSummary {
                            id: node_id,
                            name: record.name,
                            parent_id: record.parent,
                            output,
                            report: record.report,
                        },
                    )
                })
            })
            .collect();

        WorkflowRunReport {
            run_id: self.run_id,
            nodes,
        }
    }
}

impl<R, I, O, F, E, Select> StepNode<R, I, O, F, E, Select> {
    /// Creates a dynamic node from a typed step and input selector.
    pub fn new(step: Step<R, I, O, F, E>, select_input: Select) -> Self {
        Self {
            step,
            select_input,
            build_patch: None,
        }
    }

    /// Builds downstream work from the successful typed output.
    pub fn spawn_with<Build>(mut self, build_patch: Build) -> Self
    where
        Build: Fn(&NodeContext, &O) -> GraphPatch<R, E> + 'static,
    {
        self.build_patch = Some(Arc::new(build_patch));
        self
    }
}

impl<R, I, O, F, E, Select> WorkflowNode for StepNode<R, I, O, F, E, Select>
where
    R: 'static,
    I: 'static,
    O: Serialize + 'static,
    F: Serialize + Clone + 'static,
    E: 'static,
    Select: Fn(&NodeInput) -> Result<I, InputSelectionError> + 'static,
{
    type Runtime = R;
    type Error = E;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        context: NodeContext,
        input: NodeInput,
    ) -> NodeFuture<'a, Self::Runtime, Self::Error> {
        Box::pin(async move {
            let typed_input = (self.select_input)(&input)?;
            let traced = match self.step.run_traced(runtime, typed_input).await {
                Ok(traced) => traced,
                Err(StepError::System { stage, error }) => {
                    return Err(NodeExecutionError::StepSystem { stage, error });
                }
                Err(StepError::Rejected(report)) => {
                    let attempts = report.attempt_count();
                    let report =
                        encode_step_report(&report).map_err(NodeExecutionError::EncodeFindings)?;
                    return Err(NodeExecutionError::StepRejected { report, attempts });
                }
            };

            let (output, report) = traced.into_parts();
            let patch = self
                .build_patch
                .as_ref()
                .map_or_else(GraphPatch::new, |build_patch| {
                    build_patch(&context, &output)
                });
            let output = serde_json::to_value(output).map_err(NodeExecutionError::EncodeOutput)?;
            let report = encode_step_report(&report).map_err(NodeExecutionError::EncodeFindings)?;

            Ok(NodeOutcome::new(output)
                .with_patch(patch)
                .with_report(NodeReport::Step(report)))
        })
    }
}

impl<R, I, O, E, Select> StepNode<R, I, O, NeverFinding, E, Select> {
    /// Creates a dynamic node from a typed step that does not emit findings.
    pub fn without_findings(step: Step<R, I, O, NeverFinding, E>, select_input: Select) -> Self {
        Self::new(step, select_input)
    }
}

fn encode_step_report<F>(report: &StepReport<F>) -> Result<StepReport<Value>, serde_json::Error>
where
    F: Serialize + Clone,
{
    let attempts = report
        .attempts()
        .iter()
        .map(|attempt| {
            Ok(AttemptReport {
                findings: attempt
                    .findings
                    .iter()
                    .cloned()
                    .map(serde_json::to_value)
                    .collect::<Result<Vec<_>, _>>()?,
                accepted: attempt.accepted,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(StepReport::new(attempts))
}
