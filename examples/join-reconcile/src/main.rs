//! Demonstrates parallel fan-out with `.join()` and reconciliation with
//! `.reconcile_task()`.
//!
//! Two design tasks run in parallel against the same cloned input. Their
//! outputs are combined by a merge task.

use std::fmt::{Display, Formatter};

use naaf_core::{Step, task_fn};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectReport {
    plan: ProjectPlan,
    api: ApiDesign,
    ui: UiDesign,
}

#[derive(Debug)]
struct PlannerRuntime;

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
    let runtime = PlannerRuntime;

    let design_api = Step::task(|_runtime: &PlannerRuntime, input: ProjectPlan| {
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
    });

    let design_ui = Step::task(|_runtime: &PlannerRuntime, input: ProjectPlan| {
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
    });

    let merge_report = task_fn(|_runtime: &PlannerRuntime, input: (ApiDesign, UiDesign)| {
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
    });

    let combined = design_api.join(design_ui).reconcile_task(merge_report);

    let plan = ProjectPlan {
        name: "Search Feature".to_string(),
        phases: vec![
            "Implement Search".to_string(),
            "Implement Indexing".to_string(),
        ],
        estimated_weeks: 3,
    };

    let result = combined
        .run(&runtime, plan)
        .await
        .expect("parallel composition should succeed");

    println!("Plan: {}", result.plan.name);
    println!("API endpoints: {:?}", result.api.endpoints);
    println!("UI components: {:?}", result.ui.components);
}
