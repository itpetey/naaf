use std::fmt::{Display, Formatter};
use std::time::Duration;

use naaf_core::{Attempt, RetryPolicy, Step, check_fn, repair_last_fn, task_fn};
use naaf_tui::{TuiAppBuilder, TuiEvent};

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

#[tokio::main]
async fn main() {
    let (sender, handle, instruction_rx) = TuiAppBuilder::default()
        .title("tui-demo: step-retry")
        .with_input_screen("Enter project description")
        .max_log_lines(500)
        .spawn_with_input()
        .expect("TUI should spawn");

    sender
        .send(TuiEvent::Log {
            level: tracing::Level::INFO,
            target: "tui-demo".to_string(),
            message: "Waiting for instruction...".to_string(),
        })
        .ok();

    let instruction = instruction_rx
        .await
        .expect("instruction should be received");

    sender
        .send(TuiEvent::Log {
            level: tracing::Level::INFO,
            target: "tui-demo".to_string(),
            message: format!("Received: {instruction}"),
        })
        .ok();

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

    let plan_step = Step::builder(plan_project)
        .validate(review_plan)
        .repair_with(revise_plan)
        .retry_policy(RetryPolicy::new(5))
        .build();

    let brief = PlanningInput {
        name: instruction,
        goals: vec!["Ingestion".to_string()],
        estimated_weeks: 1,
    };

    tokio::time::sleep(Duration::from_millis(300)).await;

    let result = plan_step.run(&runtime, brief).await;

    match result {
        Ok(plan) => {
            sender
                .send(TuiEvent::Log {
                    level: tracing::Level::INFO,
                    target: "tui-demo".to_string(),
                    message: format!(
                        "Done! Plan: {} phases, {} weeks",
                        plan.phases.len(),
                        plan.estimated_weeks
                    ),
                })
                .ok();
        }
        Err(e) => {
            sender
                .send(TuiEvent::Log {
                    level: tracing::Level::ERROR,
                    target: "tui-demo".to_string(),
                    message: format!("Step failed: {e}"),
                })
                .ok();
        }
    }

    tokio::time::sleep(Duration::from_secs(3)).await;

    handle.shutdown().await.ok();
}
