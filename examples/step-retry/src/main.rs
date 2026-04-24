//! Demonstrates a step that validates its output and retries with a repair
//! planner until the checks pass.
//!
//! The plan task produces a plan, the check verifies it has enough phases and
//! weeks, and the repair planner generates a new input that addresses the
//! findings. The step retries up to three times.

use std::fmt::{Display, Formatter};

use naaf_core::{Attempt, RetryPolicy, Step, check_fn, repair_last_fn, task_fn};

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

    let review_plan = check_fn(
        |runtime: &PlannerRuntime, _input: PlanningInput, subject: ProjectPlan| {
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
        },
    );

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
                    estimated_weeks: last.output.estimated_weeks + runtime.repair_increment,
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
        name: "Data Platform".to_string(),
        goals: vec!["Ingestion".to_string()],
        estimated_weeks: 1,
    };

    let traced = plan_step
        .run_traced(&runtime, brief)
        .await
        .expect("plan should succeed with repair");

    println!("Plan: {}", traced.output().name);
    println!("Phases: {:?}", traced.output().phases);
    println!("Estimated weeks: {}", traced.output().estimated_weeks);
    println!("Attempts: {}", traced.report().attempt_count());
    for (i, attempt) in traced.report().attempts().iter().enumerate() {
        println!(
            "  Attempt {}: {} findings, accepted={}",
            i + 1,
            attempt.findings.len(),
            attempt.accepted
        );
    }
}
