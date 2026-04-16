//! Demonstrates a full composed workflow that sequences a validated planning
//! step with a parallel fan-out into API and UI design, reconciled into a
//! final report.
//!
//! `PlanProject` validates and repairs until the plan is accepted. Its output
//! is then fanned out in parallel to `DesignApi` and `DesignUi`, which are
//! reconciled by `MergeReport`.

use std::fmt::{Display, Formatter};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, RepairPlanner, RetryPolicy, Step, Task};

#[derive(Debug)]
struct PlannerRuntime {
    min_phases: usize,
    min_weeks: usize,
    repair_increment: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PlanningInput {
    name: String,
    goals: Vec<String>,
    estimated_weeks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectPlan {
    name: String,
    phases: Vec<String>,
    estimated_weeks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApiDesign {
    plan_name: String,
    endpoints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct UiDesign {
    plan_name: String,
    components: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectReport {
    plan: ProjectPlan,
    api: ApiDesign,
    ui: UiDesign,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Finding {
    InsufficientPhases { min: usize, actual: usize },
    EstimationTooLow { min: usize, actual: usize },
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

struct ReviewPlan;

impl Check for ReviewPlan {
    type Runtime = PlannerRuntime;
    type Subject = ProjectPlan;
    type Finding = Finding;
    type Error = Error;

    fn check<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        subject: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        Box::pin(async move {
            let mut findings = Vec::new();
            if subject.phases.len() < runtime.min_phases {
                findings.push(Finding::InsufficientPhases {
                    min: runtime.min_phases,
                    actual: subject.phases.len(),
                });
            }
            if subject.estimated_weeks < runtime.min_weeks {
                findings.push(Finding::EstimationTooLow {
                    min: runtime.min_weeks,
                    actual: subject.estimated_weeks,
                });
            }
            Ok(findings)
        })
    }
}

struct RevisePlan;

impl RepairPlanner for RevisePlan {
    type Runtime = PlannerRuntime;
    type Input = PlanningInput;
    type Artefact = ProjectPlan;
    type Finding = Finding;
    type Error = Error;

    fn repair<'a>(
        &'a self,
        runtime: &'a Self::Runtime,
        attempts: Vec<Attempt<Self::Input, Self::Artefact, Self::Finding>>,
    ) -> LocalBoxFuture<'a, Result<Self::Input, Self::Error>> {
        Box::pin(async move {
            let previous = attempts.last().expect("attempt present");
            let mut goals = previous.input.goals.clone();
            let mut extra = false;
            for finding in &previous.findings {
                match finding {
                    Finding::InsufficientPhases { .. } => {
                        goals.push("Integration testing".to_string());
                        extra = true;
                    }
                    Finding::EstimationTooLow { .. } => {
                        extra = true;
                    }
                }
            }
            if !extra {
                goals.push("Integration testing".to_string());
            }
            Ok(PlanningInput {
                name: previous.input.name.clone(),
                goals,
                estimated_weeks: previous.artefact.estimated_weeks + runtime.repair_increment,
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
    let runtime = PlannerRuntime {
        min_phases: 2,
        min_weeks: 3,
        repair_increment: 2,
    };

    let plan_step = Step::builder(PlanProject)
        .validate(ReviewPlan)
        .repair_with(RevisePlan)
        .retry_policy(RetryPolicy::new(5))
        .build();

    let api_step = Step::builder(DesignApi).with_findings::<Finding>().build();
    let ui_step = Step::builder(DesignUi).with_findings::<Finding>().build();

    let design_step = api_step.join(ui_step);
    let full_workflow = plan_step.then(design_step.reconcile_task(MergeReport));

    let brief = PlanningInput {
        name: "Monitoring System".to_string(),
        goals: vec!["Metrics collection".to_string()],
        estimated_weeks: 1,
    };

    let result = full_workflow
        .run(&runtime, brief)
        .await
        .expect("full workflow should succeed");

    println!("Report plan: {}", result.plan.name);
    println!("API endpoints: {:?}", result.api.endpoints);
    println!("UI components: {:?}", result.ui.components);
}
