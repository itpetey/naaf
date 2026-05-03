//! Demonstrates parallel fan-out with `Pipeline` and `Route::Parallel`.
//!
//! Two design tasks run in parallel against the same cloned input. Their
//! outputs are joined by the pipeline and merged into a report.

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("planner error")
    }
}

#[derive(Clone)]
struct SeedPlan;
impl Phase for SeedPlan {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = ProjectPlan;
    type Error = Error;

    fn run<'a>(
        &'a self,
        _rt: &'a PlannerRuntime,
        input: ProjectPlan,
    ) -> LocalBoxFuture<'a, Result<ProjectPlan, Error>> {
        Box::pin(async move { Ok(input) })
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

    let plan = ProjectPlan {
        name: "Search Feature".to_string(),
        phases: vec![
            "Implement Search".to_string(),
            "Implement Indexing".to_string(),
        ],
        estimated_weeks: 3,
    };

    let pipeline = Pipeline::builder()
        .add_phase(PhaseId::new("seed"), SeedPlan)
        .add_phase(PhaseId::new("design_api"), DesignApi)
        .add_phase(PhaseId::new("design_ui"), DesignUi)
        .add_phase(PhaseId::new("merge"), MergeReport)
        .with_route(
            PhaseId::new("seed"),
            Route::parallel(["design_api", "design_ui"]),
        )
        .with_route(PhaseId::new("design_api"), Route::Halt)
        .with_route(PhaseId::new("design_ui"), Route::Halt)
        .with_route(PhaseId::new("merge"), Route::Halt)
        .with_parallel_join("seed", "merge")
        .with_initial(PhaseId::new("seed"))
        .build()
        .unwrap();

    let result: ProjectReport = pipeline
        .run(&runtime, plan)
        .await
        .expect("pipeline should succeed");

    println!("Plan: {}", result.plan.name);
    println!("API endpoints: {:?}", result.api.endpoints);
    println!("UI components: {:?}", result.ui.components);
}
