use std::{
    convert::Infallible,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use futures::future::LocalBoxFuture;
use naaf_core::{EdgeSpec, GraphPatch, NodeContext, NodeId, NodeSpec, WorkflowNode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::message::ToolSpec;
use crate::tool::Tool;

/// JSON-friendly declaration of one node to spawn.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnNode {
    /// Public name used to look up the node template.
    pub name: String,
    /// Optional seed input serialised as JSON.
    #[serde(default)]
    pub seed: Option<Value>,
}

/// JSON-friendly declaration of one directed edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnEdge {
    /// Whether the edge originates from the calling (parent) node.
    #[serde(default)]
    pub from_parent: bool,
    /// Index of the source node within this request's node list.
    /// Only meaningful when `from_parent` is `false`.
    #[serde(default)]
    pub from_node: Option<usize>,
    /// Index of the target node within this request's node list.
    pub to: usize,
}

/// A complete spawn request recorded by one tool call.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Nodes to add to the workflow graph.
    pub nodes: Vec<SpawnNode>,
    /// Edges connecting the new nodes to each other and to the parent.
    #[serde(default)]
    pub edges: Vec<SpawnEdge>,
}

/// Shared accumulator for spawn requests issued during one LLM execution.
#[derive(Clone, Default)]
pub struct SpawnStore {
    requests: Arc<Mutex<Vec<SpawnRequest>>>,
}

impl SpawnStore {
    /// Creates an empty spawn store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one spawn request.
    pub fn push(&self, request: SpawnRequest) {
        self.requests
            .lock()
            .expect("spawn store lock")
            .push(request);
    }

    /// Removes and returns all accumulated requests.
    pub fn take(&self) -> Vec<SpawnRequest> {
        std::mem::take(&mut *self.requests.lock().expect("spawn store lock"))
    }
}

/// A tool that records spawn requests into a shared store.
///
/// The tool always succeeds and returns a confirmation JSON to the model.
/// Invalid arguments return an error message as JSON rather than failing.
pub struct SpawnTool<R> {
    store: SpawnStore,
    _marker: PhantomData<R>,
}

impl<R> SpawnTool<R> {
    /// Creates a spawn tool that writes into the given store.
    pub fn new(store: &SpawnStore) -> Self {
        Self {
            store: store.clone(),
            _marker: PhantomData,
        }
    }
}

impl<R> Tool for SpawnTool<R> {
    type Runtime = R;
    type Error = Infallible;

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "spawn_subtasks".to_string(),
            description: "Spawn new subtasks into the workflow graph. Each call declares nodes and edges to add.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "nodes": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "Template name for the new node",
                                },
                                "seed": {
                                    "description": "Optional seed input for the new node",
                                },
                            },
                            "required": ["name"],
                        },
                    },
                    "edges": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "from_parent": {
                                    "type": "boolean",
                                    "description": "Whether the edge originates from the calling node",
                                },
                                "from_node": {
                                    "type": "integer",
                                    "description": "Index of the source node in this request's node list",
                                },
                                "to": {
                                    "type": "integer",
                                    "description": "Index of the target node in this request's node list",
                                },
                            },
                            "required": ["to"],
                        },
                    },
                },
                "required": ["nodes"],
            }),
        }
    }

    fn call<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        arguments: Value,
    ) -> LocalBoxFuture<'a, Result<Value, Self::Error>> {
        let result = match serde_json::from_value::<SpawnRequest>(arguments) {
            Ok(request) => {
                let node_count = request.nodes.len();
                self.store.push(request);
                serde_json::json!({
                    "status": "recorded",
                    "node_count": node_count,
                })
            }
            Err(error) => serde_json::json!({
                "status": "error",
                "message": error.to_string(),
            }),
        };
        Box::pin(async move { Ok(result) })
    }
}

/// Errors produced while resolving spawn requests into a graph patch.
#[derive(Debug, Error)]
pub enum SpawnResolveError {
    /// A requested node name has no registered template.
    #[error("no template registered for node name '{name}'")]
    UnknownTemplate {
        /// Name requested by the spawn payload.
        name: String,
    },
    /// An edge referenced a node index outside the request's node list.
    #[error("edge references node index {index} but the request has {count} node(s)")]
    InvalidNodeIndex {
        /// The invalid node index referenced by the edge.
        index: usize,
        /// Total number of nodes available in the request.
        count: usize,
    },
    /// An edge that does not originate from the parent has no source node index.
    #[error("edge at index {edge_index} is missing from_node (from_parent is false)")]
    MissingFromNode {
        /// Position of the invalid edge in the request.
        edge_index: usize,
    },
}

/// Looks up a node runner by name and returns it for graph patch construction.
///
/// Implementers map a template name to a specific node runner, typically by
/// looking up a pre-built step wrapped in a [`naaf_core::StepNode`].
pub trait NodeTemplate<R, E> {
    /// Returns a shared runner for the given template name, or `None`.
    fn lookup(
        &self,
        name: &str,
        context: &NodeContext,
    ) -> Option<Arc<dyn WorkflowNode<Runtime = R, Error = E>>>;
}

impl<R, E, F> NodeTemplate<R, E> for F
where
    F: Fn(&str, &NodeContext) -> Option<Arc<dyn WorkflowNode<Runtime = R, Error = E>>>,
{
    fn lookup(
        &self,
        name: &str,
        context: &NodeContext,
    ) -> Option<Arc<dyn WorkflowNode<Runtime = R, Error = E>>> {
        self(name, context)
    }
}

/// Converts accumulated spawn requests into an additive graph patch.
///
/// Each request is resolved independently. Node identifiers are generated
/// automatically. All nodes in a request are parented on `parent_id`. Edges
/// reference nodes by their position in the request's node list.
pub fn resolve_spawn<R, E>(
    parent_id: NodeId,
    requests: Vec<SpawnRequest>,
    template: &dyn NodeTemplate<R, E>,
    context: &NodeContext,
) -> Result<GraphPatch<R, E>, SpawnResolveError> {
    let mut patch = GraphPatch::new();

    for request in requests {
        let count = request.nodes.len();
        let mut ids = Vec::with_capacity(count);

        for spawn_node in &request.nodes {
            let runner = template.lookup(&spawn_node.name, context).ok_or(
                SpawnResolveError::UnknownTemplate {
                    name: spawn_node.name.clone(),
                },
            )?;

            let mut spec = NodeSpec::from_shared_runner(spawn_node.name.clone(), runner)
                .with_parent(parent_id);

            if let Some(seed) = &spawn_node.seed {
                spec = spec.with_seed_value(seed.clone());
            }

            ids.push(spec.id());
            patch = patch.with_node(spec);
        }

        for (edge_index, edge) in request.edges.iter().enumerate() {
            if edge.to >= count {
                return Err(SpawnResolveError::InvalidNodeIndex {
                    index: edge.to,
                    count,
                });
            }

            let from_id = if edge.from_parent {
                parent_id
            } else {
                let from_index = edge
                    .from_node
                    .ok_or(SpawnResolveError::MissingFromNode { edge_index })?;
                if from_index >= count {
                    return Err(SpawnResolveError::InvalidNodeIndex {
                        index: from_index,
                        count,
                    });
                }
                ids[from_index]
            };

            let to_id = ids[edge.to];
            patch = patch.with_edge(EdgeSpec::new(from_id, to_id));
        }
    }

    Ok(patch)
}

#[cfg(test)]
mod tests {
    use naaf_core::{NeverFinding, NodeId, NodeInput, Step, StepNode, Task, WorkflowRunId};
    use serde_json::json;

    use super::*;

    #[derive(Debug)]
    struct StubRuntime;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct StubError;

    impl std::fmt::Display for StubError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("stub error")
        }
    }

    impl std::error::Error for StubError {}

    struct Increment;

    impl Task for Increment {
        type Runtime = StubRuntime;
        type Input = usize;
        type Output = usize;
        type Error = StubError;

        fn run<'a>(
            &'a self,
            _runtime: &'a Self::Runtime,
            input: Self::Input,
        ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
            Box::pin(async move { Ok(input + 1) })
        }
    }

    fn increment_runner() -> Arc<dyn WorkflowNode<Runtime = StubRuntime, Error = StubError>> {
        Arc::new(StepNode::without_findings(
            Step::builder(Increment)
                .with_findings::<NeverFinding>()
                .build(),
            |input: &NodeInput| input.seed_as::<usize>(),
        ))
    }

    #[tokio::test]
    async fn spawn_tool_records_valid_requests() {
        let store = SpawnStore::new();
        let tool = SpawnTool::<StubRuntime>::new(&store);

        let request = json!({
            "nodes": [
                {"name": "increment"},
                {"name": "double"}
            ],
            "edges": [
                {"from_parent": true, "to": 0},
                {"from_node": 0, "to": 1}
            ]
        });

        let result = tool.call(&StubRuntime, request).await.unwrap();
        assert_eq!(result["status"], "recorded");
        assert_eq!(result["node_count"], 2);

        let requests = store.take();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].nodes.len(), 2);
        assert_eq!(requests[0].edges.len(), 2);
    }

    #[tokio::test]
    async fn spawn_tool_returns_error_on_invalid_json() {
        let store = SpawnStore::new();
        let tool = SpawnTool::<StubRuntime>::new(&store);

        let result = tool
            .call(&StubRuntime, json!("not an object"))
            .await
            .unwrap();
        assert_eq!(result["status"], "error");
        assert!(store.take().is_empty());
    }

    #[test]
    fn resolve_spawn_builds_patch_from_requests() {
        let parent_id = NodeId::new();
        let context = NodeContext::new(WorkflowRunId::new(), parent_id, None);
        let runner = increment_runner();

        let template = |name: &str,
                        _: &NodeContext|
         -> Option<
            Arc<dyn WorkflowNode<Runtime = StubRuntime, Error = StubError>>,
        > {
            match name {
                "increment" => Some(runner.clone()),
                _ => None,
            }
        };

        let requests = vec![SpawnRequest {
            nodes: vec![
                SpawnNode {
                    name: "increment".to_string(),
                    seed: Some(json!(3)),
                },
                SpawnNode {
                    name: "increment".to_string(),
                    seed: Some(json!(10)),
                },
            ],
            edges: vec![
                SpawnEdge {
                    from_parent: true,
                    from_node: None,
                    to: 0,
                },
                SpawnEdge {
                    from_parent: true,
                    from_node: None,
                    to: 1,
                },
            ],
        }];

        let patch = resolve_spawn(parent_id, requests, &template, &context).unwrap();
        assert_eq!(patch.nodes().len(), 2);
        assert_eq!(patch.edges().len(), 2);
        assert_eq!(patch.edges()[0].from(), parent_id);
        assert_eq!(patch.edges()[1].from(), parent_id);
    }

    #[test]
    fn resolve_spawn_rejects_unknown_template() {
        let parent_id = NodeId::new();
        let context = NodeContext::new(WorkflowRunId::new(), parent_id, None);

        let template = |_: &str,
                        _: &NodeContext|
         -> Option<
            Arc<dyn WorkflowNode<Runtime = StubRuntime, Error = StubError>>,
        > { None };

        let requests = vec![SpawnRequest {
            nodes: vec![SpawnNode {
                name: "unknown".to_string(),
                seed: None,
            }],
            edges: vec![],
        }];

        let result = resolve_spawn(parent_id, requests, &template, &context);
        assert!(matches!(
            result,
            Err(SpawnResolveError::UnknownTemplate { .. })
        ));
    }

    #[test]
    fn resolve_spawn_rejects_invalid_edge_index() {
        let parent_id = NodeId::new();
        let context = NodeContext::new(WorkflowRunId::new(), parent_id, None);
        let runner = increment_runner();

        let template = |name: &str,
                        _: &NodeContext|
         -> Option<
            Arc<dyn WorkflowNode<Runtime = StubRuntime, Error = StubError>>,
        > {
            match name {
                "increment" => Some(runner.clone()),
                _ => None,
            }
        };

        let requests = vec![SpawnRequest {
            nodes: vec![SpawnNode {
                name: "increment".to_string(),
                seed: None,
            }],
            edges: vec![SpawnEdge {
                from_parent: true,
                from_node: None,
                to: 5,
            }],
        }];

        let result = resolve_spawn(parent_id, requests, &template, &context);
        assert!(matches!(
            result,
            Err(SpawnResolveError::InvalidNodeIndex { .. })
        ));
    }

    #[test]
    fn resolve_spawn_rejects_missing_from_node() {
        let parent_id = NodeId::new();
        let context = NodeContext::new(WorkflowRunId::new(), parent_id, None);
        let runner = increment_runner();

        let template = |name: &str,
                        _: &NodeContext|
         -> Option<
            Arc<dyn WorkflowNode<Runtime = StubRuntime, Error = StubError>>,
        > {
            match name {
                "increment" => Some(runner.clone()),
                _ => None,
            }
        };

        let requests = vec![SpawnRequest {
            nodes: vec![
                SpawnNode {
                    name: "increment".to_string(),
                    seed: None,
                },
                SpawnNode {
                    name: "increment".to_string(),
                    seed: None,
                },
            ],
            edges: vec![SpawnEdge {
                from_parent: false,
                from_node: None,
                to: 1,
            }],
        }];

        let result = resolve_spawn(parent_id, requests, &template, &context);
        assert!(matches!(
            result,
            Err(SpawnResolveError::MissingFromNode { .. })
        ));
    }
}
