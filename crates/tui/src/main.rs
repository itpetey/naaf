//! TUI for NAAF - displays workflow execution in real-time.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Parser;
use naaf_core::budget::DummyServices;
use naaf_core::events::ExecutionEvent;
use naaf_core::executor::Executor;
use naaf_schema::artifacts::ArtifactKey;
use naaf_schema::execution_status::ExecutionStatus;
use naaf_schema::lineage::Lineage;
use naaf_schema::state::{RunId, StateEnvelope, StateId};
use naaf_schema::state_kind::StateKind;
use tokio_util::sync::CancellationToken;

#[derive(Parser, Debug)]
#[command(
    name = "naaf-tui",
    about = "NAAF TUI - Interactive workflow viewer",
    version
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    #[command(about = "Run a workflow and watch execution")]
    Watch {
        #[arg(help = "Input text for the workflow")]
        input: String,
        #[arg(short, long, default_value = "draft-request", help = "Workflow to run")]
        workflow: String,
    },
    #[command(about = "Inspect a run directory")]
    Inspect {
        #[arg(help = "Run ID to inspect")]
        run_id: String,
    },
}

#[derive(Clone)]
struct TuiEventSink {
    events: Arc<Mutex<Vec<ExecutionEvent>>>,
}

impl TuiEventSink {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl naaf_core::events::TraceSink for TuiEventSink {
    fn emit(&self, event: ExecutionEvent) -> naaf_core::events::EventResult {
        println!("{}", format_event(&event));
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

fn format_event(event: &ExecutionEvent) -> String {
    match event {
        ExecutionEvent::RunStarted { timestamp, .. } => {
            format!("[{}] ▶ Run started", timestamp.format("%H:%M:%S%.3f"))
        }
        ExecutionEvent::StepEntered {
            timestamp,
            step_name,
            ..
        } => {
            format!("[{}] → {}", timestamp.format("%H:%M:%S%.3f"), step_name)
        }
        ExecutionEvent::PromptRendered { timestamp, .. } => {
            format!(
                "[{}]   📝 Prompt rendered",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::ProviderCalled { timestamp, .. } => {
            format!(
                "[{}]   🤖 Provider called",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::ProviderResponded { timestamp, .. } => {
            format!(
                "[{}]   ✓ Provider responded",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::ArtifactsParsed { timestamp, .. } => {
            format!(
                "[{}]   📦 Artifacts parsed",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::ValidatorPassed { timestamp, .. } => {
            format!(
                "[{}]   ✓ Validator passed",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::ValidatorFailed { timestamp, .. } => {
            format!(
                "[{}]   ✗ Validator failed",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::RouteSelected { timestamp, .. } => {
            format!("[{}]   ⇢ Route selected", timestamp.format("%H:%M:%S%.3f"))
        }
        ExecutionEvent::BranchStarted { timestamp, .. } => {
            format!("[{}]   ⎇ Branch started", timestamp.format("%H:%M:%S%.3f"))
        }
        ExecutionEvent::BranchCompleted { timestamp, .. } => {
            format!(
                "[{}]   ✓ Branch completed",
                timestamp.format("%H:%M:%S%.3f")
            )
        }
        ExecutionEvent::JoinReduced { timestamp, .. } => {
            format!("[{}]   ⋮ Join reduced", timestamp.format("%H:%M:%S%.3f"))
        }
        ExecutionEvent::RunTerminated { timestamp, .. } => {
            format!("[{}] ■ Run completed", timestamp.format("%H:%M:%S%.3f"))
        }
        ExecutionEvent::RunFailed {
            timestamp, error, ..
        } => {
            format!(
                "[{}] ✗ Run failed: {}",
                timestamp.format("%H:%M:%S%.3f"),
                error
            )
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Watch { input, workflow } => {
            watch_workflow(input, &workflow).await?;
        }
        Command::Inspect { run_id } => {
            inspect_run(&run_id)?;
        }
    }

    Ok(())
}

async fn watch_workflow(input: String, workflow_name: &str) -> Result<()> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║         NAAF Workflow Execution Viewer            ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Workflow: {}", workflow_name);
    println!("Input: {}", input);
    println!();
    println!("─ Executing ───────────────────────────────────────");
    println!();

    let workflow = match workflow_name {
        "draft-request" => naaf_builtins::draft_request_workflow()?,
        _ => anyhow::bail!(
            "Unknown workflow: {}. Available: draft-request",
            workflow_name
        ),
    };

    let executor = Executor::new(workflow)?;

    let mut state = StateEnvelope::new(
        StateId::new(),
        RunId::new(),
        StateKind::Proposed,
        Lineage::new(None, None, ExecutionStatus::Pending),
    );
    state.artifacts.insert(
        ArtifactKey::new("input"),
        naaf_schema::artifacts::ArtifactValue::text(&input),
    );

    let sink = TuiEventSink::new();
    let mut ctx = naaf_core::budget::ExecCtx::new(RunId::new(), DummyServices)
        .with_trace(Box::new(sink.clone()))
        .with_cancel(CancellationToken::new());

    match executor.execute(&mut ctx, state).await {
        Ok(final_state) => {
            println!();
            println!("─ Final State ──────────────────────────────────────");
            println!();
            println!("State kind: {:?}", final_state.kind);
            println!();

            if !final_state.artifacts.is_empty() {
                println!("Artifacts:");
                for (key, value) in final_state.artifacts.iter() {
                    println!("  • {}: {}", key, format_artifact_value(value));
                }
                println!();
            }

            println!("─ Execution Summary ────────────────────────────────");
            println!();
            println!("Steps executed: {}", ctx.step_count);
            println!("Branches executed: {}", ctx.branch_count);
            println!();
            println!("✓ Workflow completed successfully");
        }
        Err(e) => {
            println!();
            println!("✗ Workflow failed: {}", e);
        }
    }

    Ok(())
}

fn format_artifact_value(value: &naaf_schema::artifacts::ArtifactValue) -> String {
    if let Some(text) = value.as_text() {
        if text.len() > 60 {
            format!("{}...", &text[..57])
        } else {
            text.clone()
        }
    } else if let Some(json) = value.as_json() {
        let s = serde_json::to_string(json).unwrap_or_else(|_| "invalid json".to_string());
        if s.len() > 60 {
            format!("{}...", &s[..57])
        } else {
            s
        }
    } else {
        "unknown type".to_string()
    }
}

fn inspect_run(run_id: &str) -> Result<()> {
    let _run_uuid: uuid::Uuid = run_id.parse()?;

    let run_dir = std::path::PathBuf::from(".runs").join(run_id);
    if !run_dir.exists() {
        anyhow::bail!("Run not found: {}", run_id);
    }

    let event_file = run_dir.join("events.log");
    if !event_file.exists() {
        anyhow::bail!("No events found for run: {}", run_id);
    }

    let events = naaf_core::events::FilesystemEventStore::read_events(&event_file)?;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║         NAAF Run Inspector                        ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Run ID: {}", run_id);
    println!();

    if let Some(ExecutionEvent::RunStarted { timestamp, .. }) = events.first() {
        println!("Started: {}", timestamp.format("%Y-%m-%d %H:%M:%S"));
    }

    if let Some(ExecutionEvent::RunTerminated { timestamp, .. }) = events.last() {
        if let Some(ExecutionEvent::RunStarted {
            timestamp: start, ..
        }) = events.first()
        {
            let duration = timestamp.signed_duration_since(start);
            println!("Duration: {:?}", duration);
        }
        println!("Status: ✓ Completed");
    }

    if let Some(ExecutionEvent::RunFailed {
        timestamp: _,
        error,
        ..
    }) = events
        .iter()
        .rev()
        .find(|e| matches!(e, ExecutionEvent::RunFailed { .. }))
    {
        println!("Status: ✗ Failed");
        println!("Error: {}", error);
    }

    println!();
    println!("─ Execution Trace ──────────────────────────────────");
    println!();

    let mut step_num = 0;
    for event in &events {
        match event {
            ExecutionEvent::StepEntered { step_name, .. } => {
                step_num += 1;
                println!("{}. {}", step_num, step_name);
            }
            ExecutionEvent::RouteSelected { .. } => {
                println!("   └─ Route selected");
            }
            ExecutionEvent::BranchStarted { .. } => {
                println!("   └─ Branch started");
            }
            ExecutionEvent::ValidatorPassed { .. } => {
                println!("   └─ ✓ Validator passed");
            }
            ExecutionEvent::ValidatorFailed { .. } => {
                println!("   └─ ✗ Validator failed");
            }
            _ => {}
        }
    }

    println!();

    Ok(())
}
