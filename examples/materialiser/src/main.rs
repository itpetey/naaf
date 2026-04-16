//! Demonstrates a step that materialises its task output into a different
//! subject type before checking it.
//!
//! `PlanProject` produces a `ProjectPlan`, which `ReviewPlan` validates. Then
//! `WriteProjectPlan` materialises it into a `String` for further checks. The
//! repair planner revises the input when findings are reported.

use std::fmt::{Display, Formatter};

use futures::future::LocalBoxFuture;
use naaf_core::{Attempt, Check, Materialiser, RepairPlanner, RetryPolicy, Step, Task};

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

struct WriteProjectPlan;

impl Materialiser for WriteProjectPlan {
    type Runtime = PlannerRuntime;
    type Input = ProjectPlan;
    type Output = String;
    type Error = Error;

    fn materialise<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        input: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            Ok(format!(
                "# {name}\n\nPhases: {phases}\nEstimated: {weeks} weeks",
                name = input.name,
                phases = input.phases.join(", "),
                weeks = input.estimated_weeks,
            ))
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

#[tokio::main]
async fn main() {
    let runtime = PlannerRuntime {
        min_phases: 3,
        min_weeks: 4,
        repair_increment: 2,
    };

    let plan_step = Step::builder(PlanProject)
        .validate(ReviewPlan)
        .materialise(WriteProjectPlan)
        .repair_with(RevisePlan)
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
