use std::sync::{Arc, Mutex};

use futures::future::LocalBoxFuture;
use naaf_core::{
    EdgeSpec, GraphPatch, InvalidPatchError, NodeContext, NodeId, NodeInput, NodeReport, NodeSpec,
    Step, StepNode, Task, Workflow, WorkflowError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
struct TestRuntime {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl TestRuntime {
    fn record(&self, event: &'static str) {
        self.events.lock().expect("events lock").push(event);
    }

    fn events(&self) -> Vec<&'static str> {
        self.events.lock().expect("events lock").clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestError;

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("test error")
    }
}

impl std::error::Error for TestError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Plan {
    feature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ApiDraft {
    file: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct UiDraft {
    file: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ReleasePlan {
    artefacts: Vec<String>,
}

struct PlanFeature;

impl Task for PlanFeature {
    type Runtime = TestRuntime;
    type Input = String;
    type Output = Plan;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("plan_feature");
            Ok(Plan { feature: input })
        })
    }
}

struct GenerateApi;

impl Task for GenerateApi {
    type Runtime = TestRuntime;
    type Input = Plan;
    type Output = ApiDraft;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("generate_api");
            Ok(ApiDraft {
                file: format!("src/{}/api.rs", input.feature),
            })
        })
    }
}

struct GenerateUi;

impl Task for GenerateUi {
    type Runtime = TestRuntime;
    type Input = Plan;
    type Output = UiDraft;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("generate_ui");
            Ok(UiDraft {
                file: format!("src/{}/ui.rs", input.feature),
            })
        })
    }
}

struct ReconcileRelease;

impl Task for ReconcileRelease {
    type Runtime = TestRuntime;
    type Input = (ApiDraft, UiDraft);
    type Output = ReleasePlan;
    type Error = TestError;

    fn run<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            runtime.record("reconcile_release");
            let mut artefacts = vec![input.0.file, input.1.file];
            artefacts.sort();
            Ok(ReleasePlan { artefacts })
        })
    }
}

fn plan_step() -> Step<TestRuntime, String, Plan, (), TestError> {
    Step::builder(PlanFeature).with_findings::<()>().build()
}

fn api_step() -> Step<TestRuntime, Plan, ApiDraft, (), TestError> {
    Step::builder(GenerateApi).with_findings::<()>().build()
}

fn ui_step() -> Step<TestRuntime, Plan, UiDraft, (), TestError> {
    Step::builder(GenerateUi).with_findings::<()>().build()
}

fn reconcile_step() -> Step<TestRuntime, (ApiDraft, UiDraft), ReleasePlan, (), TestError> {
    Step::builder(ReconcileRelease)
        .with_findings::<()>()
        .build()
}

#[tokio::test]
async fn workflow_runs_spawned_sub_tasks_and_reconciles_outputs() {
    let runtime = TestRuntime::default();
    let root_id = NodeId::new();
    let api = api_step();
    let ui = ui_step();
    let reconcile = reconcile_step();

    let root = NodeSpec::new(
        "plan_feature",
        StepNode::new(plan_step(), |input: &NodeInput| input.seed_as::<String>()).spawn_with(
            move |context: &NodeContext, _plan: &Plan| {
                let planner_id = context.node_id();
                let api_id = NodeId::new();
                let ui_id = NodeId::new();
                let reconcile_id = NodeId::new();

                GraphPatch::new()
                    .with_node(
                        NodeSpec::new(
                            "generate_api",
                            StepNode::new(api.clone(), move |input: &NodeInput| {
                                input.output_as::<Plan>(planner_id)
                            }),
                        )
                        .with_id(api_id)
                        .with_parent(planner_id),
                    )
                    .with_node(
                        NodeSpec::new(
                            "generate_ui",
                            StepNode::new(ui.clone(), move |input: &NodeInput| {
                                input.output_as::<Plan>(planner_id)
                            }),
                        )
                        .with_id(ui_id)
                        .with_parent(planner_id),
                    )
                    .with_node(
                        NodeSpec::new(
                            "reconcile_release",
                            StepNode::new(reconcile.clone(), move |input: &NodeInput| {
                                Ok((
                                    input.output_as::<ApiDraft>(api_id)?,
                                    input.output_as::<UiDraft>(ui_id)?,
                                ))
                            }),
                        )
                        .with_id(reconcile_id)
                        .with_parent(planner_id),
                    )
                    .with_edge(EdgeSpec::new(planner_id, api_id))
                    .with_edge(EdgeSpec::new(planner_id, ui_id))
                    .with_edge(EdgeSpec::new(api_id, reconcile_id))
                    .with_edge(EdgeSpec::new(ui_id, reconcile_id))
            },
        ),
    )
    .with_id(root_id)
    .with_seed("search".to_string())
    .expect("seed should serialise");

    let report = Workflow::new()
        .with_max_concurrency(4)
        .with_patch(GraphPatch::new().with_node(root))
        .expect("root patch should validate")
        .run(&runtime)
        .await
        .expect("workflow should succeed");

    assert_eq!(report.nodes().len(), 4);
    let events = runtime.events();
    assert_eq!(events.len(), 4);
    assert_eq!(events.first(), Some(&"plan_feature"));
    assert_eq!(events.last(), Some(&"reconcile_release"));
    assert!(events.contains(&"generate_api"));
    assert!(events.contains(&"generate_ui"));

    let root_summary = report.node(root_id).expect("root node should exist");
    assert_eq!(root_summary.name(), "plan_feature");

    let reconcile_summary = report
        .nodes()
        .values()
        .find(|node| node.name() == "reconcile_release")
        .expect("reconcile node should exist");
    let release_plan: ReleasePlan =
        serde_json::from_value(reconcile_summary.output().clone()).expect("output should decode");
    assert_eq!(
        release_plan.artefacts,
        vec![
            "src/search/api.rs".to_string(),
            "src/search/ui.rs".to_string()
        ]
    );

    for node in report.nodes().values().filter(|node| node.id() != root_id) {
        assert_eq!(node.parent_id(), Some(root_id));
        match node.report() {
            NodeReport::Step(report) => assert_eq!(report.attempt_count(), 1),
            NodeReport::Empty => panic!("spawned step node should expose a step report"),
        }
    }
}

#[tokio::test]
async fn workflow_rejects_patch_that_targets_existing_nodes() {
    let runtime = TestRuntime::default();
    let root_id = NodeId::new();

    let root = NodeSpec::new(
        "plan_feature",
        StepNode::new(plan_step(), |input: &NodeInput| input.seed_as::<String>()).spawn_with(
            move |context: &NodeContext, _plan: &Plan| {
                let child_id = NodeId::new();
                let planner_id = context.node_id();

                GraphPatch::new()
                    .with_node(
                        NodeSpec::new(
                            "generate_api",
                            StepNode::new(api_step(), move |input: &NodeInput| {
                                input.output_as::<Plan>(planner_id)
                            }),
                        )
                        .with_id(child_id)
                        .with_parent(planner_id),
                    )
                    .with_edge(EdgeSpec::new(child_id, planner_id))
            },
        ),
    )
    .with_id(root_id)
    .with_seed("search".to_string())
    .expect("seed should serialise");

    let error = Workflow::new()
        .with_patch(GraphPatch::new().with_node(root))
        .expect("root patch should validate")
        .run(&runtime)
        .await
        .expect_err("workflow should reject invalid additive patch");

    match error {
        WorkflowError::InvalidPatch(InvalidPatchError::ExistingTarget { node_id }) => {
            assert_eq!(node_id, root_id);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
