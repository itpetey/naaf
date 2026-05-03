//! Demonstrates a full composed pipeline that sequences a validated planning
//! step with a parallel fan-out into API and UI design, reconciled into a
//! final report.
//!
//! The plan task validates and repairs until the plan is accepted. Its output
//! is then passed to design phases and merged.

use std::fmt::{Display, Formatter};

use futures::future::LocalBoxFuture;
use naaf_core::{Phase, PhaseId, Pipeline, Route};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectReport {
    plan: ProjectPlan,
    api: ApiDesign,
    ui: UiDesign,
}

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
enum DesignDraft {
    Api {
        plan: ProjectPlan,
        design: ApiDesign,
    },
    Ui {
        plan: ProjectPlan,
        design: UiDesign,
    },
}

fn plan_from_input(input: PlanningInput) -> ProjectPlan {
    ProjectPlan {
        name: input.name,
        phases: input
            .goals
            .iter()
            .map(|goal| format!("Implement {goal}"))
            .collect(),
        estimated_weeks: input.estimated_weeks,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("planner error")
    }
}

#[derive(Clone)]
struct PlanProject;
impl Phase for PlanProject {
    type Runtime = PlannerRuntime;
    type Input = PlanningInput;
    type Output = ProjectPlan;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _rt: &'a PlannerRuntime,
        input: PlanningInput,
    ) -> LocalBoxFuture<'a, Result<ProjectPlan, Error>> {
        Box::pin(async move { Ok(plan_from_input(input)) })
    }
}

#[derive(Clone)]
struct DesignApi;
impl Phase for DesignApi {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = DesignDraft;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _rt: &'a PlannerRuntime,
        input: ProjectPlan,
    ) -> LocalBoxFuture<'a, Result<DesignDraft, Error>> {
        Box::pin(async move {
            let endpoints = input
                .phases
                .iter()
                .map(|phase| format!("/api/{phase}"))
                .collect();
            Ok(DesignDraft::Api {
                plan: input.clone(),
                design: ApiDesign {
                    plan_name: input.name,
                    endpoints,
                },
            })
        })
    }
}

#[derive(Clone)]
struct DesignUi;
impl Phase for DesignUi {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = DesignDraft;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _rt: &'a PlannerRuntime,
        input: ProjectPlan,
    ) -> LocalBoxFuture<'a, Result<DesignDraft, Error>> {
        Box::pin(async move {
            let components = input
                .phases
                .iter()
                .map(|phase| format!("{phase}Panel"))
                .collect();
            Ok(DesignDraft::Ui {
                plan: input.clone(),
                design: UiDesign {
                    plan_name: input.name,
                    components,
                },
            })
        })
    }
}

#[derive(Clone)]
struct MergeReport;
impl Phase for MergeReport {
    type Runtime = PlannerRuntime;
    type Input = Vec<DesignDraft>;
    type Output = ProjectReport;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _rt: &'a PlannerRuntime,
        drafts: Vec<DesignDraft>,
    ) -> LocalBoxFuture<'a, Result<ProjectReport, Error>> {
        Box::pin(async move {
            let mut plan = None;
            let mut api = None;
            let mut ui = None;
            for draft in drafts {
                match draft {
                    DesignDraft::Api { plan: p, design } => {
                        plan = Some(p);
                        api = Some(design);
                    }
                    DesignDraft::Ui { plan: p, design } => {
                        plan.get_or_insert(p);
                        ui = Some(design);
                    }
                }
            }
            Ok(ProjectReport {
                plan: plan.expect("plan should be present"),
                api: api.expect("api design should be present"),
                ui: ui.expect("ui design should be present"),
            })
        })
    }
}

#[tokio::main]
async fn main() {
    let runtime = PlannerRuntime;

    let brief = PlanningInput {
        name: "Data Platform".to_string(),
        goals: vec!["Ingestion".to_string()],
        estimated_weeks: 1,
    };

    let pipeline = Pipeline::builder()
        .add_phase(PhaseId::new("plan"), PlanProject)
        .add_phase(PhaseId::new("design_api"), DesignApi)
        .add_phase(PhaseId::new("design_ui"), DesignUi)
        .add_phase(PhaseId::new("merge"), MergeReport)
        .with_route(
            PhaseId::new("plan"),
            Route::parallel(["design_api", "design_ui"]),
        )
        .with_route(PhaseId::new("design_ui"), Route::Halt)
        .with_route(PhaseId::new("design_api"), Route::Halt)
        .with_route(PhaseId::new("merge"), Route::Halt)
        .with_parallel_join("plan", "merge")
        .with_initial(PhaseId::new("plan"))
        .build()
        .unwrap();

    let result: ProjectReport = pipeline
        .run(&runtime, brief)
        .await
        .expect("pipeline should succeed");

    println!("Plan: {}", result.plan.name);
    println!("Phases: {:?}", result.plan.phases);
    println!("API endpoints: {:?}", result.api.endpoints);
    println!("UI components: {:?}", result.ui.components);
}
