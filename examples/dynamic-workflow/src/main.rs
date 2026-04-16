//! Demonstrates dynamic workflow graph construction using `Workflow`,
//! `StepNode`, `NodeSpec`, `GraphPatch`, and `EdgeSpec`.
//!
//! A root planning step spawns three downstream nodes (two parallel design
//! steps and a merge step) at runtime via `spawn_with`. The workflow engine
//! executes the graph, routing outputs through edges.

use std::fmt::{Display, Formatter};

use futures::future::LocalBoxFuture;
use naaf_core::{
    EdgeSpec, GraphPatch, NeverFinding, NodeContext, NodeId, NodeInput, NodeReport, NodeSpec, Step,
    StepNode, Task, Workflow,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct PlannerRuntime;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PlanningInput {
    name: String,
    goals: Vec<String>,
    estimated_weeks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectPlan {
    name: String,
    phases: Vec<String>,
    estimated_weeks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ApiDesign {
    plan_name: String,
    endpoints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct UiDesign {
    plan_name: String,
    components: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectReport {
    plan: ProjectPlan,
    api: ApiDesign,
    ui: UiDesign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("planner error")
    }
}

impl std::error::Error for Error {}

struct PlanProject;

impl Task for PlanProject {
    type Runtime = PlannerRuntime;
    type Input = PlanningInput;
    type Output = ProjectPlan;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let phases = input
                .goals
                .iter()
                .map(|goal| format!("Implement {goal}"))
                .collect();
            Ok(ProjectPlan {
                name: input.name,
                phases,
                estimated_weeks: input.estimated_weeks,
            })
        })
    }
}

struct DesignApi;

impl Task for DesignApi {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = ApiDesign;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let endpoints = input
                .phases
                .iter()
                .map(|phase| format!("/api/{phase}"))
                .collect();
            Ok(ApiDesign {
                plan_name: input.name,
                endpoints,
            })
        })
    }
}

struct DesignUi;

impl Task for DesignUi {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = UiDesign;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let components = input
                .phases
                .iter()
                .map(|phase| format!("{phase}Panel"))
                .collect();
            Ok(UiDesign {
                plan_name: input.name,
                components,
            })
        })
    }
}

struct MergeReport;

impl Task for MergeReport {
    type Runtime = PlannerRuntime;
    type Input = (ApiDesign, UiDesign);
    type Output = ProjectReport;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let plan = ProjectPlan {
                name: input.0.plan_name.clone(),
                phases: Vec::new(),
                estimated_weeks: 0,
            };
            Ok(ProjectReport {
                plan,
                api: input.0,
                ui: input.1,
            })
        })
    }
}

#[tokio::main]
async fn main() {
    let runtime = PlannerRuntime;

    let root_id = NodeId::new();
    let plan_step: Step<PlannerRuntime, PlanningInput, ProjectPlan, NeverFinding, Error> =
        Step::builder(PlanProject)
            .with_findings::<NeverFinding>()
            .build();
    let api_step = Step::builder(DesignApi)
        .with_findings::<NeverFinding>()
        .build();
    let ui_step = Step::builder(DesignUi)
        .with_findings::<NeverFinding>()
        .build();
    let merge_step = Step::builder(MergeReport)
        .with_findings::<NeverFinding>()
        .build();

    let api_step_clone = api_step.clone();
    let ui_step_clone = ui_step.clone();
    let merge_step_clone = merge_step.clone();

    let root = NodeSpec::new(
        "plan_project",
        StepNode::new(plan_step, |input: &NodeInput| {
            input.seed_as::<PlanningInput>()
        })
        .spawn_with(move |context: &NodeContext, _plan: &ProjectPlan| {
            let planner_id = context.node_id();
            let api_id = NodeId::new();
            let ui_id = NodeId::new();
            let merge_id = NodeId::new();

            GraphPatch::new()
                .with_node(
                    NodeSpec::new(
                        "design_api",
                        StepNode::new(api_step_clone.clone(), move |input: &NodeInput| {
                            input.output_as::<ProjectPlan>(planner_id)
                        }),
                    )
                    .with_id(api_id)
                    .with_parent(planner_id),
                )
                .with_node(
                    NodeSpec::new(
                        "design_ui",
                        StepNode::new(ui_step_clone.clone(), move |input: &NodeInput| {
                            input.output_as::<ProjectPlan>(planner_id)
                        }),
                    )
                    .with_id(ui_id)
                    .with_parent(planner_id),
                )
                .with_node(
                    NodeSpec::new(
                        "merge_report",
                        StepNode::new(merge_step_clone.clone(), move |input: &NodeInput| {
                            let api = input.output_as::<ApiDesign>(api_id)?;
                            let ui = input.output_as::<UiDesign>(ui_id)?;
                            Ok((api, ui))
                        }),
                    )
                    .with_id(merge_id)
                    .with_parent(planner_id),
                )
                .with_edge(EdgeSpec::new(planner_id, api_id))
                .with_edge(EdgeSpec::new(planner_id, ui_id))
                .with_edge(EdgeSpec::new(api_id, merge_id))
                .with_edge(EdgeSpec::new(ui_id, merge_id))
        }),
    )
    .with_id(root_id)
    .with_seed(PlanningInput {
        name: "Event System".to_string(),
        goals: vec!["Event sourcing".to_string(), "Event replay".to_string()],
        estimated_weeks: 4,
    })
    .expect("seed should serialise");

    let report = Workflow::new()
        .with_max_concurrency(4)
        .with_patch(GraphPatch::new().with_node(root))
        .expect("root patch should validate")
        .run(&runtime)
        .await
        .expect("workflow should succeed");

    for (id, node) in report.nodes() {
        println!("Node: {} ({id})", node.name());
        if let NodeReport::Step(step_report) = node.report() {
            println!("  Attempts: {}", step_report.attempt_count());
        }
    }

    let merge_node = report
        .nodes()
        .values()
        .find(|node| node.name() == "merge_report")
        .expect("merge node should exist");
    let merged: ProjectReport =
        serde_json::from_value(merge_node.output().clone()).expect("output should decode");
    println!("Merged report plan: {}", merged.plan.name);
    println!("API endpoints: {:?}", merged.api.endpoints);
    println!("UI components: {:?}", merged.ui.components);
}
