//! Demonstrates a full composed workflow that sequences a validated planning
//! step with a parallel fan-out into API and UI design, reconciled into a
//! final report.
//!
//! The plan task validates and repairs until the plan is accepted. Its output
//! is then fanned out in parallel to design the API and UI, which are
//! reconciled by a merge task.

use std::fmt::{Display, Formatter};

use naaf_core::{Attempt, RetryPolicy, Step, check_fn, repair_last_fn, task_fn};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectReport {
    plan: ProjectPlan,
    api: ApiDesign,
    ui: UiDesign,
}

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
enum Finding {
    InsufficientPhases { min: usize, actual: usize },
    EstimationTooLow { min: usize, actual: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error;

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("planner error")
    }
}

#[tokio::main]
async fn main() {
    let runtime = PlannerRuntime {
        min_phases: 3,
        min_weeks: 4,
        repair_increment: 2,
    };

    let plan_project = task_fn(|_runtime: &PlannerRuntime, input: PlanningInput| {
        let phases = input
            .goals
            .iter()
            .map(|goal| format!("Implement {goal}"))
            .collect();
        Box::pin(async move {
            Ok::<_, Error>(ProjectPlan {
                name: input.name,
                phases,
                estimated_weeks: input.estimated_weeks,
            })
        })
    });

    let review_plan = check_fn(|runtime: &PlannerRuntime, subject: ProjectPlan| {
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
        Box::pin(async move { Ok::<_, Error>(findings) })
    });

    let revise_plan = repair_last_fn(
        |runtime: &PlannerRuntime, last: Attempt<PlanningInput, ProjectPlan, Finding>| {
            let mut goals = last.input.goals;
            let mut extra = false;
            for finding in &last.findings {
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
            Box::pin(async move {
                Ok::<_, Error>(PlanningInput {
                    name: last.input.name,
                    goals,
                    estimated_weeks: last.artefact.estimated_weeks + runtime.repair_increment,
                })
            })
        },
    );

    let design_api = Step::builder(task_fn(|_runtime: &PlannerRuntime, input: ProjectPlan| {
        let endpoints = input
            .phases
            .iter()
            .map(|phase| format!("/api/{phase}"))
            .collect();
        Box::pin(async move {
            Ok::<_, Error>(ApiDesign {
                plan_name: input.name,
                endpoints,
            })
        })
    }))
    .with_findings::<Finding>()
    .build();

    let design_ui = Step::builder(task_fn(|_runtime: &PlannerRuntime, input: ProjectPlan| {
        let components = input
            .phases
            .iter()
            .map(|phase| format!("{phase}Panel"))
            .collect();
        Box::pin(async move {
            Ok::<_, Error>(UiDesign {
                plan_name: input.name,
                components,
            })
        })
    }))
    .with_findings::<Finding>()
    .build();

    let plan_step = Step::builder(plan_project)
        .validate(review_plan)
        .repair_with(revise_plan)
        .retry_policy(RetryPolicy::new(5))
        .build();

    let workflow = plan_step.then(design_api.join(design_ui).reconcile_task(task_fn(
        |_runtime: &PlannerRuntime, input: (ApiDesign, UiDesign)| {
            let plan = ProjectPlan {
                name: input.0.plan_name.clone(),
                phases: Vec::new(),
                estimated_weeks: 0,
            };
            Box::pin(async move {
                Ok::<_, Error>(ProjectReport {
                    plan,
                    api: input.0,
                    ui: input.1,
                })
            })
        },
    )));

    let brief = PlanningInput {
        name: "Data Platform".to_string(),
        goals: vec!["Ingestion".to_string()],
        estimated_weeks: 1,
    };

    let result = workflow
        .run(&runtime, brief)
        .await
        .expect("composed workflow should succeed");

    println!("Plan: {}", result.plan.name);
    println!("Phases: {:?}", result.plan.phases);
    println!("API endpoints: {:?}", result.api.endpoints);
    println!("UI components: {:?}", result.ui.components);
}
