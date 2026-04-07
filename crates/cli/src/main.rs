//! CLI entry point.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use naaf_core::budget::{DummyServices, ExecCtx};
use naaf_core::events::ExecutionEvent;
use naaf_core::executor::Executor;
use naaf_core::state_store::StateStore;
use naaf_openspec::draft_request_workflow;
use naaf_schema::artifacts::{ArtifactKey, ArtifactValue};
use naaf_schema::execution_status::ExecutionStatus;
use naaf_schema::lineage::Lineage;
use naaf_schema::state::{RunId, StateEnvelope, StateId};
use naaf_schema::state_kind::StateKind;
use tokio_util::sync::CancellationToken;

const RUNS_DIR: &str = ".runs";

struct RunSummary {
    run_id: RunId,
    ambiguous_escalation: bool,
}

#[derive(Parser, Debug)]
#[command(name = "naaf", about = "NAAF - Not Another AI Framework CLI", version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    #[command(about = "Run a workflow with input")]
    Run {
        #[arg(help = "Workflow to run")]
        workflow: String,
        #[arg(help = "Input text for the workflow")]
        input: String,
    },
    #[command(about = "List all runs")]
    List,
    #[command(about = "Show execution trace for a run")]
    Trace {
        #[arg(help = "Run ID to trace")]
        run_id: String,
    },
    #[command(about = "Inspect final state of a run")]
    Inspect {
        #[arg(help = "Run ID to inspect")]
        run_id: String,
    },
    #[command(about = "Replay a run with the same input")]
    Replay {
        #[arg(help = "Run ID to replay")]
        run_id: String,
    },
    #[command(about = "List available workflows")]
    Workflows,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Run { workflow, input } => {
            run(&workflow, input).await?;
        }
        Command::List => {
            list_runs()?;
        }
        Command::Trace { run_id } => {
            trace_run(&run_id)?;
        }
        Command::Inspect { run_id } => {
            inspect_run(&run_id)?;
        }
        Command::Replay { run_id } => {
            replay_run(&run_id).await?;
        }
        Command::Workflows => {
            list_workflows()?;
        }
    }

    Ok(())
}

async fn run(workflow_name: &str, input: String) -> Result<()> {
    let interactive_clarification = io::stdin().is_terminal() && io::stdout().is_terminal();
    let initial_run = run_once(workflow_name, &input).await?;

    if !interactive_clarification || !initial_run.ambiguous_escalation {
        return Ok(());
    }

    println!("\nThis request needs clarification before it can continue.");
    println!("Original ambiguous run: {}", initial_run.run_id);

    let Some(clarification) = prompt_for_clarification()? else {
        println!("No clarification provided. Leaving the ambiguous run as-is.");
        return Ok(());
    };

    let clarified_input = compose_clarified_input(&input, &clarification);
    println!("\nStarting clarified follow-up run...");
    let follow_up_run = run_once(workflow_name, &clarified_input).await?;

    println!("\nOriginal ambiguous run: {}", initial_run.run_id);
    println!("Clarified follow-up run: {}", follow_up_run.run_id);

    Ok(())
}

async fn run_once(workflow_name: &str, input: &str) -> Result<RunSummary> {
    let runs_dir = PathBuf::from(RUNS_DIR);
    std::fs::create_dir_all(&runs_dir)?;

    let run_id = RunId::new();
    let run_dir = runs_dir.join(run_id.to_string());
    std::fs::create_dir_all(&run_dir)?;

    let event_file = run_dir.join("events.log");
    let event_sink = naaf_core::events::FilesystemEventStore::new(&event_file)?;

    println!("Running workflow: {}", workflow_name);
    println!("Run ID: {}", run_id);
    println!("Input: {}", input);

    let workflow = match workflow_name {
        "draft-request" => draft_request_workflow()?,
        _ => anyhow::bail!(
            "Unknown workflow: {}. Available: draft-request",
            workflow_name
        ),
    };

    let executor = Executor::new(workflow)?;

    let mut state = StateEnvelope::new(
        StateId::new(),
        run_id,
        StateKind::Proposed,
        Lineage::new(None, None, ExecutionStatus::Pending),
    );
    state
        .artifacts
        .insert(ArtifactKey::new("input"), ArtifactValue::text(input));

    let mut ctx = ExecCtx::new(run_id, DummyServices)
        .with_trace(Box::new(event_sink))
        .with_cancel(CancellationToken::new());

    println!("\nExecuting...");

    let mut ambiguous_escalation = false;

    match executor.execute(&mut ctx, state).await {
        Ok(final_state) => {
            ambiguous_escalation = is_ambiguous_escalation(&final_state);

            if let Err(e) = StateStore::save(&final_state, &run_dir) {
                eprintln!("Warning: Failed to persist state: {}", e);
            }

            println!("\n✓ Workflow completed successfully");
            println!("\nFinal state kind: {:?}", final_state.kind);

            if let Some(response) = final_state.artifacts.get(&ArtifactKey::new("response"))
                && let Some(text) = response.as_text()
            {
                println!("Response: {}", text);
            }

            if let Some(acceptance) = final_state.artifacts.get(&ArtifactKey::new("acceptance"))
                && let Some(json) = acceptance.as_json()
            {
                println!("Acceptance: {}", serde_json::to_string_pretty(json)?);
            }

            if let Some(escalation) = final_state.artifacts.get(&ArtifactKey::new("escalation"))
                && let Some(json) = escalation.as_json()
            {
                println!("\nEscalation:");
                println!("{}", serde_json::to_string_pretty(json)?);
            }
        }
        Err(e) => {
            println!("\n✗ Workflow failed: {}", e);
        }
    }

    println!("\nRun directory: {}", run_dir.display());

    Ok(RunSummary {
        run_id,
        ambiguous_escalation,
    })
}

fn is_ambiguous_escalation(final_state: &StateEnvelope) -> bool {
    final_state
        .artifacts
        .get(&ArtifactKey::new("escalation"))
        .and_then(|escalation| escalation.as_json())
        .is_some_and(escalation_is_ambiguous)
}

fn escalation_is_ambiguous(escalation: &serde_json::Value) -> bool {
    escalation
        .get("classification")
        .and_then(|classification| classification.as_str())
        == Some("Ambiguous")
}

fn prompt_for_clarification() -> Result<Option<String>> {
    print!("Clarification: ");
    io::stdout().flush()?;

    let mut clarification = String::new();
    if io::stdin().read_line(&mut clarification)? == 0 {
        return Ok(None);
    }

    let clarification = clarification.trim().to_string();
    if clarification.is_empty() {
        return Ok(None);
    }

    Ok(Some(clarification))
}

fn compose_clarified_input(input: &str, clarification: &str) -> String {
    format!(
        "build this request using the following clarification:\n\nOriginal request:\n{}\n\nClarification:\n{}",
        input, clarification
    )
}

fn list_runs() -> Result<()> {
    let runs_dir = PathBuf::from(RUNS_DIR);
    if !runs_dir.exists() {
        println!("No runs found");
        return Ok(());
    }

    let entries = std::fs::read_dir(&runs_dir)?;
    let mut runs: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    runs.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    if runs.is_empty() {
        println!("No runs found");
        return Ok(());
    }

    println!(
        "{:<38} {:<15} {:<12} STARTED",
        "RUN ID", "STATE KIND", "STATUS"
    );
    println!("{}", "-".repeat(90));

    for entry in runs {
        let run_id_str = entry.file_name().to_string_lossy().into_owned();
        let run_dir = entry.path();

        if run_id_str.parse::<uuid::Uuid>().is_err() {
            continue;
        }

        let event_file = run_dir.join("events.log");
        if !event_file.exists() {
            continue;
        }

        let events = naaf_core::events::FilesystemEventStore::read_events(&event_file)?;

        let first_event = events.first();
        let last_event = events.last();

        let state_kind = events
            .iter()
            .rev()
            .find_map(|e| match e {
                ExecutionEvent::StepEntered { step_name, .. } => Some(step_name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "unknown".to_string());

        let status = match last_event {
            Some(ExecutionEvent::RunTerminated { .. }) => "done",
            Some(ExecutionEvent::RunFailed { .. }) => "failed",
            _ => "running",
        };

        let started = first_event.and_then(|e| match e {
            ExecutionEvent::RunStarted { timestamp, .. } => Some(timestamp),
            _ => None,
        });

        let started_str = started
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<38} {:<15} {:<12} {}",
            run_id_str, state_kind, status, started_str
        );
    }

    Ok(())
}

fn trace_run(run_id: &str) -> Result<()> {
    let _run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;

    let run_dir = PathBuf::from(RUNS_DIR).join(run_id);
    if !run_dir.exists() {
        anyhow::bail!("Run not found: {}", run_id);
    }

    let event_file = run_dir.join("events.log");
    if !event_file.exists() {
        anyhow::bail!("No events found for run: {}", run_id);
    }

    let events = naaf_core::events::FilesystemEventStore::read_events(&event_file)?;

    if events.is_empty() {
        println!("No events in run");
        return Ok(());
    }

    println!("Run: {}", run_id);
    println!("{}\n", "=".repeat(90));

    let mut step_count = 0;
    let mut prompt_count = 0;
    let mut provider_call_count = 0;

    for event in &events {
        match event {
            ExecutionEvent::RunStarted { timestamp, .. } => {
                println!("{}  Run started", timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
            ExecutionEvent::StepEntered {
                timestamp,
                step_name,
                ..
            } => {
                step_count += 1;
                println!(
                    "{}  → Entered step: {}",
                    timestamp.format("%Y-%m-%d %H:%M:%S"),
                    step_name
                );
            }
            ExecutionEvent::PromptRendered { timestamp, .. } => {
                prompt_count += 1;
                println!(
                    "{}    Prompt rendered",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::ProviderCalled { timestamp, .. } => {
                provider_call_count += 1;
                println!(
                    "{}    Provider called",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::ProviderResponded { timestamp, .. } => {
                println!(
                    "{}    Provider responded",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::ArtifactsParsed { timestamp, .. } => {
                println!(
                    "{}    Artifacts parsed",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::ValidatorPassed { timestamp, .. } => {
                println!(
                    "{}    ✓ Validator passed",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::ValidatorFailed { timestamp, .. } => {
                println!(
                    "{}    ✗ Validator failed",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::RouteSelected { timestamp, .. } => {
                println!(
                    "{}    Route selected",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::BranchStarted { timestamp, .. } => {
                println!(
                    "{}    Branch started",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::BranchCompleted { timestamp, .. } => {
                println!(
                    "{}    Branch completed",
                    timestamp.format("%Y-%m-%d %H:%M:%S")
                );
            }
            ExecutionEvent::JoinReduced { timestamp, .. } => {
                println!("{}    Join reduced", timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
            ExecutionEvent::RunTerminated { timestamp, .. } => {
                println!("{}  Run completed", timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
            ExecutionEvent::RunFailed {
                timestamp, error, ..
            } => {
                println!(
                    "{}  Run failed: {}",
                    timestamp.format("%Y-%m-%d %H:%M:%S"),
                    error
                );
            }
        }
    }

    println!("\n{}", "=".repeat(90));
    println!("Summary:");
    println!("  Steps executed: {}", step_count);
    println!("  Prompts rendered: {}", prompt_count);
    println!("  Provider calls: {}", provider_call_count);

    Ok(())
}

fn inspect_run(run_id: &str) -> Result<()> {
    let _run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;

    let run_dir = PathBuf::from(RUNS_DIR).join(run_id);
    if !run_dir.exists() {
        anyhow::bail!("Run not found: {}", run_id);
    }

    let event_file = run_dir.join("events.log");
    if !event_file.exists() {
        anyhow::bail!("No events found for run: {}", run_id);
    }

    let events = naaf_core::events::FilesystemEventStore::read_events(&event_file)?;

    println!("Run: {}", run_id);
    println!("Directory: {}", run_dir.display());
    println!();

    let started = events.first().and_then(|e| match e {
        ExecutionEvent::RunStarted { timestamp, .. } => Some(*timestamp),
        _ => None,
    });

    let terminated = events.iter().rev().find_map(|e| match e {
        ExecutionEvent::RunTerminated { timestamp, .. } => Some(*timestamp),
        _ => None,
    });

    let failed = events.iter().rev().find_map(|e| match e {
        ExecutionEvent::RunFailed {
            timestamp, error, ..
        } => Some((*timestamp, error.clone())),
        _ => None,
    });

    if let Some(start) = started {
        println!("Started: {}", start.format("%Y-%m-%d %H:%M:%S"));
    }

    match StateStore::load(&run_dir) {
        Ok(state) => {
            println!("State kind: {:?}", state.kind);
            println!();

            if let Some(start) = started
                && let Some(end) = terminated
            {
                let duration = end.signed_duration_since(start);
                println!("Duration: {:?}", duration);
            }

            let status = if terminated.is_some() {
                "✓ Completed"
            } else if failed.is_some() {
                "✗ Failed"
            } else {
                "Running"
            };
            println!("Status: {}", status);

            println!();
            println!("─ Artifacts ─────────────────────────────────────────");
            println!();

            for (key, value) in state.artifacts.iter() {
                println!("  {}", key);
                if let Some(text) = value.as_text() {
                    if text.len() > 100 {
                        println!("    {}", &text[..100]);
                        println!("    ... ({} bytes total)", text.len());
                    } else {
                        println!("    {}", text);
                    }
                } else if let Some(json) = value.as_json() {
                    let s = serde_json::to_string_pretty(json)?;
                    for line in s.lines().take(10) {
                        println!("    {}", line);
                    }
                    if s.lines().count() > 10 {
                        println!("    ... ({} lines total)", s.lines().count());
                    }
                }
            }
        }
        Err(_) => {
            if let Some(end) = terminated {
                if let Some(start) = started {
                    let duration = end.signed_duration_since(start);
                    println!("Duration: {:?}", duration);
                }
                println!("Status: ✓ Completed");
            }

            if let Some((_, error)) = failed {
                println!("Status: ✗ Failed");
                println!("Error: {}", error);
            }
        }
    }

    let steps: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ExecutionEvent::StepEntered { step_name, .. } => Some(step_name.clone()),
            _ => None,
        })
        .collect();

    println!();
    println!("─ Execution Trace ───────────────────────────────────");
    println!();
    println!("Steps executed ({}):", steps.len());
    for (i, step) in steps.iter().enumerate() {
        println!("  {}. {}", i + 1, step);
    }

    Ok(())
}

fn list_workflows() -> Result<()> {
    println!("Available workflows:");
    println!();
    println!("  draft-request");
    println!("    Draft request workflow with classification, normalization, and acceptance");
    println!();
    println!("Usage: naaf run <workflow> <input>");
    println!("Example: naaf run draft-request \"Create a file\"");

    Ok(())
}

async fn replay_run(run_id: &str) -> Result<()> {
    let _run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;

    let run_dir = PathBuf::from(RUNS_DIR).join(run_id);
    if !run_dir.exists() {
        anyhow::bail!("Run not found: {}", run_id);
    }

    let previous_state = StateStore::load(&run_dir).context("Failed to load previous run state")?;

    let input = previous_state
        .artifacts
        .get(&ArtifactKey::new("input"))
        .and_then(|v| v.as_text())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No input artifact found in previous run"))?;

    println!("Replaying run: {}", run_id);
    println!("Original input: {}", input);
    println!();

    run_once("draft-request", &input).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalation_classification_detects_ambiguity() {
        let escalation = serde_json::json!({
            "classification": "Ambiguous",
            "confidence": 0.6,
        });

        assert!(escalation_is_ambiguous(&escalation));
    }

    #[test]
    fn escalation_classification_ignores_other_values() {
        let escalation = serde_json::json!({
            "classification": "Actionable",
        });

        assert!(!escalation_is_ambiguous(&escalation));
    }

    #[test]
    fn clarification_input_is_composed_as_structured_text() {
        let clarified = compose_clarified_input("plan a todo app", "single-user React app");

        assert!(clarified.starts_with("build "));
        assert!(clarified.contains("Original request:\nplan a todo app"));
        assert!(clarified.contains("Clarification:\nsingle-user React app"));
    }
}
