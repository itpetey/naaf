//! Workflow-aware TUI host for NAAF.

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::Parser;
use naaf_core::budget::{DummyServices, ExecCtx};
use naaf_core::events::{ExecutionEvent, TraceSink};
use naaf_core::executor::Executor;
use naaf_core::state_store::StateStore;
use naaf_core::workflow_package::DiscoveredWorkflowPackage;
use naaf_core::{WorkflowRegistry, build_workflow, discover_workflow_packages};
use naaf_schema::artifacts::{ArtifactKey, ArtifactValue};
use naaf_schema::execution_status::ExecutionStatus;
use naaf_schema::lineage::Lineage;
use naaf_schema::state::{RunId, StateEnvelope, StateId};
use naaf_schema::state_kind::StateKind;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

const DEFAULT_RUNS_DIR_NAME: &str = ".runs";
const DEFAULT_WORKFLOWS_DIR_NAME: &str = "workflows";
const RUN_RECORD_FILE: &str = "run.json";

#[derive(Parser, Debug)]
#[command(name = "naaf-tui", about = "NAAF workflow host", version)]
struct Args {
    #[arg(
        long,
        env = "NAAF_WORKFLOWS_DIR",
        global = true,
        help = "Override the workflow packages directory"
    )]
    workflows_dir: Option<PathBuf>,
    #[arg(
        long,
        env = "NAAF_RUNS_DIR",
        global = true,
        help = "Override the run storage directory"
    )]
    runs_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug)]
struct HostPaths {
    workflows_dir: PathBuf,
    runs_dir: PathBuf,
}

#[derive(Parser, Debug)]
enum Command {
    #[command(about = "List discovered workflow packages")]
    Workflows,
    #[command(about = "Run a discovered workflow package")]
    Run {
        #[arg(help = "Workflow package identifier")]
        workflow: String,
        #[arg(help = "Input text for the workflow")]
        input: Option<String>,
    },
    #[command(about = "List saved workflow runs")]
    Runs,
    #[command(about = "Inspect a saved run")]
    Inspect {
        #[arg(help = "Run ID to inspect")]
        run_id: String,
    },
    #[command(about = "Replay a saved run")]
    Replay {
        #[arg(help = "Run ID to replay")]
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

struct HostTraceSink {
    console: TuiEventSink,
    file: naaf_core::events::FilesystemEventStore,
}

impl naaf_core::events::TraceSink for HostTraceSink {
    fn emit(&self, event: ExecutionEvent) -> naaf_core::events::EventResult {
        self.file.emit(event.clone())?;
        self.console.emit(event)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RunRecord {
    workflow: String,
    input: String,
}

struct RunSummary {
    run_id: RunId,
    ambiguous_escalation: bool,
}

fn resolve_host_paths(args: &Args) -> Result<HostPaths> {
    let current_dir = std::env::current_dir().context("Failed to resolve current directory")?;
    let current_exe = std::env::current_exe().ok();
    resolve_host_paths_from(
        args.workflows_dir.clone(),
        args.runs_dir.clone(),
        &current_dir,
        current_exe.as_deref(),
    )
}

fn resolve_host_paths_from(
    workflows_override: Option<PathBuf>,
    runs_override: Option<PathBuf>,
    current_dir: &Path,
    current_exe: Option<&Path>,
) -> Result<HostPaths> {
    let repo_root = find_repo_root(current_dir, current_exe).ok();

    let workflows_dir = workflows_override
        .clone()
        .or_else(|| {
            runs_override
                .as_ref()
                .and_then(|runs_dir| runs_dir.parent())
                .map(|parent| parent.join(DEFAULT_WORKFLOWS_DIR_NAME))
        })
        .or_else(|| {
            repo_root
                .as_ref()
                .map(|root| root.join(DEFAULT_WORKFLOWS_DIR_NAME))
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not locate the repository workflows directory. Run from the repo, set --workflows-dir / NAAF_WORKFLOWS_DIR, or pair --runs-dir with a sibling workflows directory."
            )
        })?;

    let runs_dir = runs_override
        .clone()
        .or_else(|| {
            workflows_override
                .as_ref()
                .and_then(|workflows_dir| workflows_dir.parent())
                .map(|parent| parent.join(DEFAULT_RUNS_DIR_NAME))
        })
        .or_else(|| {
            repo_root
                .as_ref()
                .map(|root| root.join(DEFAULT_RUNS_DIR_NAME))
        })
        .unwrap_or_else(|| current_dir.join(DEFAULT_RUNS_DIR_NAME));

    Ok(HostPaths {
        workflows_dir,
        runs_dir,
    })
}

fn find_repo_root(current_dir: &Path, current_exe: Option<&Path>) -> Result<PathBuf> {
    for base in candidate_roots(current_dir, current_exe) {
        if let Some(root) = search_ancestors_for_workflows(&base) {
            return Ok(root);
        }
    }

    anyhow::bail!(
        "Could not locate the repository workflows directory. Run from the repo, or set --workflows-dir / NAAF_WORKFLOWS_DIR."
    )
}

fn candidate_roots(current_dir: &Path, current_exe: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = vec![current_dir.to_path_buf()];
    if let Some(current_exe) = current_exe {
        candidates.push(current_exe.to_path_buf());
    }
    candidates
}

fn search_ancestors_for_workflows(start: &Path) -> Option<PathBuf> {
    let mut cursor = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };

    while let Some(path) = cursor {
        if path.join(DEFAULT_WORKFLOWS_DIR_NAME).is_dir() {
            return Some(path.to_path_buf());
        }
        cursor = path.parent();
    }

    None
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let paths = resolve_host_paths(&args)?;

    match args.command {
        Command::Workflows => list_workflows(&paths)?,
        Command::Run { workflow, input } => run_workflow_host(&paths, &workflow, input).await?,
        Command::Runs => list_runs(&paths)?,
        Command::Inspect { run_id } => inspect_run(&paths, &run_id)?,
        Command::Replay { run_id } => replay_run(&paths, &run_id).await?,
    }

    Ok(())
}

async fn run_workflow_host(
    paths: &HostPaths,
    workflow_id: &str,
    input: Option<String>,
) -> Result<()> {
    let packages = load_workflow_packages(paths)?;
    let package = find_workflow(&packages, workflow_id)?;
    let input = match input {
        Some(input) => input,
        None => prompt_for_input(&package.package.ui.input_prompt)?
            .ok_or_else(|| anyhow::anyhow!("No input provided"))?,
    };

    let interactive_clarification = io::stdin().is_terminal() && io::stdout().is_terminal();
    let initial_run = run_once(paths, package, &input).await?;
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
    let follow_up_run = run_once(paths, package, &clarified_input).await?;

    println!("\nOriginal ambiguous run: {}", initial_run.run_id);
    println!("Clarified follow-up run: {}", follow_up_run.run_id);

    Ok(())
}

async fn run_once(
    paths: &HostPaths,
    package: &DiscoveredWorkflowPackage,
    input: &str,
) -> Result<RunSummary> {
    std::fs::create_dir_all(&paths.runs_dir)?;

    let run_id = RunId::new();
    let run_dir = paths.runs_dir.join(run_id.to_string());
    std::fs::create_dir_all(&run_dir)?;

    save_run_record(
        &run_dir,
        &RunRecord {
            workflow: package.package.id.clone(),
            input: input.to_string(),
        },
    )?;

    let event_file = run_dir.join("events.log");
    let event_sink = naaf_core::events::FilesystemEventStore::new(&event_file)?;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║               NAAF Workflow Host                ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!(
        "Workflow: {} ({})",
        package.package.name, package.package.id
    );
    println!("Input: {}", input);
    println!("Run ID: {}", run_id);
    println!();
    println!("─ Executing ───────────────────────────────────────");
    println!();

    let input_artifact = ArtifactKey::new(&package.package.ui.input_artifact);
    let initial_state = make_initial_state(run_id, input_artifact, input);

    let mut registry = WorkflowRegistry::<DummyServices>::new();
    naaf_openspec::register_workflow_steps(&mut registry);

    let workflow = match build_workflow(&package.package, &registry) {
        Ok(workflow) => workflow,
        Err(error) => {
            record_failed_run(&event_sink, run_id, &initial_state, &error)?;
            StateStore::save(&initial_state, &run_dir)?;
            println!();
            println!("✗ Workflow failed: {}", error);
            println!();
            println!("Run directory: {}", run_dir.display());
            return Err(anyhow::Error::new(error));
        }
    };
    let executor = match Executor::new(workflow) {
        Ok(executor) => executor,
        Err(error) => {
            record_failed_run(&event_sink, run_id, &initial_state, &error)?;
            StateStore::save(&initial_state, &run_dir)?;
            println!();
            println!("✗ Workflow failed: {}", error);
            println!();
            println!("Run directory: {}", run_dir.display());
            return Err(anyhow::Error::new(error));
        }
    };

    let sink = TuiEventSink::new();
    let mut ctx = ExecCtx::new(run_id, DummyServices)
        .with_trace(Box::new(HostTraceSink {
            console: sink.clone(),
            file: event_sink,
        }))
        .with_cancel(CancellationToken::new());

    let ambiguous_escalation = match executor.execute(&mut ctx, initial_state).await {
        Ok(final_state) => {
            let ambiguous_escalation = is_ambiguous_escalation(&final_state);
            StateStore::save(&final_state, &run_dir)?;

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
            ambiguous_escalation
        }
        Err(error) => {
            if let Some(latest_state) = ctx.latest_state() {
                StateStore::save(&latest_state, &run_dir)?;
            }
            println!();
            println!("✗ Workflow failed: {}", error);
            println!();
            println!("Run directory: {}", run_dir.display());
            return Err(anyhow::Error::new(error));
        }
    };

    println!();
    println!("Run directory: {}", run_dir.display());

    Ok(RunSummary {
        run_id,
        ambiguous_escalation,
    })
}

fn list_workflows(paths: &HostPaths) -> Result<()> {
    let packages = load_workflow_packages(paths)?;
    if packages.is_empty() {
        println!(
            "No workflow packages found in `{}`",
            paths.workflows_dir.display()
        );
        return Ok(());
    }

    println!("Available workflows:");
    println!();
    for package in &packages {
        println!("  {}", package.package.id);
        println!("    {}", package.package.name);
        if !package.package.summary.is_empty() {
            println!("    {}", package.package.summary);
        }
        println!();
    }

    Ok(())
}

fn list_runs(paths: &HostPaths) -> Result<()> {
    list_runs_in(&paths.runs_dir)
}

fn list_runs_in(runs_dir: &Path) -> Result<()> {
    if !runs_dir.exists() {
        println!("No runs found");
        return Ok(());
    }

    let mut runs: Vec<_> = std::fs::read_dir(runs_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect();

    runs.sort_by(|left, right| {
        let left_time = left
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok();
        let right_time = right
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok();
        right_time.cmp(&left_time)
    });

    if runs.is_empty() {
        println!("No runs found");
        return Ok(());
    }

    println!(
        "{:<38} {:<18} {:<12} STARTED",
        "RUN ID", "WORKFLOW", "STATUS"
    );
    println!("{}", "-".repeat(95));

    for entry in runs {
        let run_id_str = entry.file_name().to_string_lossy().into_owned();
        if run_id_str.parse::<uuid::Uuid>().is_err() {
            continue;
        }

        let run_dir = entry.path();
        let event_file = run_dir.join("events.log");
        if !event_file.exists() {
            continue;
        }

        let events = naaf_core::events::FilesystemEventStore::read_events(&event_file)?;
        let record = load_run_record(&run_dir).ok();
        let status = match events.last() {
            Some(ExecutionEvent::RunTerminated { .. }) => "done",
            Some(ExecutionEvent::RunFailed { .. }) => "failed",
            _ => "running",
        };
        let started = events.first().and_then(|event| match event {
            ExecutionEvent::RunStarted { timestamp, .. } => Some(timestamp),
            _ => None,
        });
        let started_str = started
            .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<38} {:<18} {:<12} {}",
            run_id_str,
            record
                .as_ref()
                .map(|record| record.workflow.as_str())
                .unwrap_or("<unknown>"),
            status,
            started_str
        );
    }

    Ok(())
}

fn inspect_run(paths: &HostPaths, run_id: &str) -> Result<()> {
    inspect_run_in(&paths.runs_dir, run_id)
}

fn inspect_run_in(runs_dir: &Path, run_id: &str) -> Result<()> {
    let _run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;

    let run_dir = runs_dir.join(run_id);
    if !run_dir.exists() {
        anyhow::bail!("Run not found: {}", run_id);
    }

    let event_file = run_dir.join("events.log");
    if !event_file.exists() {
        anyhow::bail!("No events found for run: {}", run_id);
    }

    let events = naaf_core::events::FilesystemEventStore::read_events(&event_file)?;
    let state = StateStore::load(&run_dir).ok();
    let record = load_run_record(&run_dir).ok();

    println!("╔══════════════════════════════════════════════════╗");
    println!("║                NAAF Run Inspector               ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Run ID: {}", run_id);
    if let Some(record) = &record {
        println!("Workflow: {}", record.workflow);
        println!("Input: {}", record.input);
    }
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

    if let Some(ExecutionEvent::RunFailed { error, .. }) = events
        .iter()
        .rev()
        .find(|event| matches!(event, ExecutionEvent::RunFailed { .. }))
    {
        println!("Status: ✗ Failed");
        println!("Error: {}", error);
    }

    println!();
    println!("─ Final State ──────────────────────────────────────");
    println!();
    if let Some(state) = &state {
        println!("State kind: {:?}", state.kind);
        println!();
        if !state.artifacts.is_empty() {
            for (key, value) in state.artifacts.iter() {
                println!("  {}", key);
                println!("    {}", format_artifact_value(value));
            }
        }
    } else {
        println!("No saved state found");
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
            ExecutionEvent::RouteSelected { .. } => println!("   └─ Route selected"),
            ExecutionEvent::BranchStarted { .. } => println!("   └─ Branch started"),
            ExecutionEvent::ValidatorPassed { .. } => println!("   └─ ✓ Validator passed"),
            ExecutionEvent::ValidatorFailed { .. } => println!("   └─ ✗ Validator failed"),
            _ => {}
        }
    }

    Ok(())
}

async fn replay_run(paths: &HostPaths, run_id: &str) -> Result<()> {
    let _run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;

    let run_dir = paths.runs_dir.join(run_id);
    if !run_dir.exists() {
        anyhow::bail!("Run not found: {}", run_id);
    }

    let record = load_run_record(&run_dir)?;

    println!("Replaying run: {}", run_id);
    println!("Workflow: {}", record.workflow);
    println!("Original input: {}", record.input);
    println!();

    run_workflow_host(paths, &record.workflow, Some(record.input)).await
}

fn load_workflow_packages(paths: &HostPaths) -> Result<Vec<DiscoveredWorkflowPackage>> {
    Ok(discover_workflow_packages(&paths.workflows_dir)?)
}

fn find_workflow<'a>(
    packages: &'a [DiscoveredWorkflowPackage],
    workflow_id: &str,
) -> Result<&'a DiscoveredWorkflowPackage> {
    packages
        .iter()
        .find(|package| package.package.id == workflow_id)
        .ok_or_else(|| {
            let available = packages
                .iter()
                .map(|package| package.package.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::anyhow!(
                "Unknown workflow '{}'. Available workflows: {}",
                workflow_id,
                available
            )
        })
}

fn make_initial_state(run_id: RunId, input_key: ArtifactKey, input: &str) -> StateEnvelope {
    let mut state = StateEnvelope::new(
        StateId::new(),
        run_id,
        StateKind::Proposed,
        Lineage::new(None, None, ExecutionStatus::Pending),
    );
    state
        .artifacts
        .insert(input_key, ArtifactValue::text(input.to_string()));
    state
}

fn record_failed_run(
    event_sink: &naaf_core::events::FilesystemEventStore,
    run_id: RunId,
    state: &StateEnvelope,
    error: &naaf_core::errors::Error,
) -> Result<()> {
    event_sink.emit(ExecutionEvent::RunStarted {
        run_id,
        state_id: state.id,
        step_name: "workflow".to_string(),
        sequence_number: 0,
        timestamp: chrono::Utc::now(),
    })?;
    event_sink.emit(ExecutionEvent::RunFailed {
        run_id,
        state_id: state.id,
        step_name: "workflow".to_string(),
        error: error.to_string(),
        sequence_number: 1,
        timestamp: chrono::Utc::now(),
    })?;
    Ok(())
}

fn save_run_record(run_dir: &Path, record: &RunRecord) -> Result<()> {
    let path = run_dir.join(RUN_RECORD_FILE);
    let json = serde_json::to_string_pretty(record)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn load_run_record(run_dir: &Path) -> Result<RunRecord> {
    let path = run_dir.join(RUN_RECORD_FILE);
    if !path.exists() {
        anyhow::bail!(
            "Run metadata not found at {}. Replay requires run.json for this development build.",
            path.display()
        );
    }

    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

fn prompt_for_input(prompt: &str) -> Result<Option<String>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(None);
    }

    let prompt = if prompt.trim().is_empty() {
        "Input"
    } else {
        prompt
    };

    print!("{}: ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Ok(None);
    }

    let input = input.trim().to_string();
    if input.is_empty() {
        return Ok(None);
    }

    Ok(Some(input))
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

fn format_artifact_value(value: &ArtifactValue) -> String {
    if let Some(text) = value.as_text() {
        if text.len() > 80 {
            format!("{}...", &text[..77])
        } else {
            text.clone()
        }
    } else if let Some(json) = value.as_json() {
        let rendered = serde_json::to_string(json).unwrap_or_else(|_| "invalid json".to_string());
        if rendered.len() > 80 {
            format!("{}...", &rendered[..77])
        } else {
            rendered
        }
    } else {
        "unknown type".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use naaf_core::workflow_package::{WorkflowPackage, WorkflowPackageUi};
    use tempfile::tempdir;

    fn demo_package(id: &str) -> DiscoveredWorkflowPackage {
        DiscoveredWorkflowPackage {
            root_dir: PathBuf::from(format!("/tmp/{id}")),
            manifest_path: PathBuf::from(format!("/tmp/{id}/workflow.toml")),
            package: WorkflowPackage {
                id: id.to_string(),
                name: format!("{id} workflow"),
                summary: "summary".to_string(),
                entry: "start".to_string(),
                ui: WorkflowPackageUi {
                    input_artifact: "input".to_string(),
                    input_prompt: "Describe the request".to_string(),
                },
                nodes: vec![],
                edges: vec![],
            },
        }
    }

    fn make_repo_layout() -> tempfile::TempDir {
        let temp = tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(DEFAULT_WORKFLOWS_DIR_NAME)).unwrap();
        temp
    }

    fn write_event_log(run_dir: &Path, run_id: RunId, state_id: StateId) {
        let started = ExecutionEvent::RunStarted {
            run_id,
            state_id,
            step_name: "workflow".to_string(),
            sequence_number: 0,
            timestamp: chrono::Utc::now(),
        };
        let terminated = ExecutionEvent::RunTerminated {
            run_id,
            state_id,
            step_name: "workflow".to_string(),
            sequence_number: 1,
            timestamp: chrono::Utc::now(),
        };
        let payload = format!(
            "{}\n{}\n",
            serde_json::to_string(&started).unwrap(),
            serde_json::to_string(&terminated).unwrap()
        );
        std::fs::write(run_dir.join("events.log"), payload).unwrap();
    }

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

    #[test]
    fn saves_and_loads_run_record() {
        let temp = tempdir().unwrap();
        let record = RunRecord {
            workflow: "draft-request".to_string(),
            input: "create a file".to_string(),
        };

        save_run_record(temp.path(), &record).unwrap();
        let loaded = load_run_record(temp.path()).unwrap();
        assert_eq!(loaded.workflow, record.workflow);
        assert_eq!(loaded.input, record.input);
    }

    #[test]
    fn selects_workflow_by_identifier() {
        let packages = vec![demo_package("draft-request"), demo_package("other")];

        let found = find_workflow(&packages, "other").unwrap();
        assert_eq!(found.package.id, "other");
    }

    #[test]
    fn requires_run_record_when_metadata_is_missing() {
        let temp = tempdir().unwrap();
        let error = load_run_record(temp.path()).unwrap_err();
        assert!(error.to_string().contains("Run metadata not found"));
    }

    #[test]
    fn inspects_saved_run_from_custom_runs_directory() {
        let temp = tempdir().unwrap();
        let run_id = RunId::new();
        let run_dir = temp.path().join(run_id.to_string());
        std::fs::create_dir_all(&run_dir).unwrap();

        let state_id = StateId::new();
        let mut state = StateEnvelope::new(
            state_id,
            run_id,
            StateKind::Accepted,
            Lineage::new(None, None, ExecutionStatus::Succeeded),
        );
        state.artifacts.insert(
            ArtifactKey::new("input"),
            ArtifactValue::text("inspect me".to_string()),
        );
        StateStore::save(&state, &run_dir).unwrap();
        save_run_record(
            &run_dir,
            &RunRecord {
                workflow: "draft-request".to_string(),
                input: "inspect me".to_string(),
            },
        )
        .unwrap();
        write_event_log(&run_dir, run_id, state_id);

        let result = inspect_run_in(temp.path(), &run_id.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn resolves_repo_paths_from_current_directory() {
        let temp = make_repo_layout();
        let nested = temp.path().join("workflows").join("openspec");

        let paths = resolve_host_paths_from(None, None, &nested, None).unwrap();
        assert_eq!(
            paths.workflows_dir,
            temp.path().join(DEFAULT_WORKFLOWS_DIR_NAME)
        );
        assert_eq!(paths.runs_dir, temp.path().join(DEFAULT_RUNS_DIR_NAME));
    }

    #[test]
    fn resolves_repo_paths_from_executable_location() {
        let temp = make_repo_layout();
        let exe = temp.path().join("target").join("debug").join("naaf-tui");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "").unwrap();

        let outside = tempdir().unwrap();
        let paths =
            resolve_host_paths_from(None, None, outside.path(), Some(exe.as_path())).unwrap();

        assert_eq!(
            paths.workflows_dir,
            temp.path().join(DEFAULT_WORKFLOWS_DIR_NAME)
        );
        assert_eq!(paths.runs_dir, temp.path().join(DEFAULT_RUNS_DIR_NAME));
    }

    #[test]
    fn cli_override_takes_precedence_for_host_paths() {
        let temp = make_repo_layout();
        let workflows_override = temp.path().join("custom-workflows");
        let runs_override = temp.path().join("custom-runs");

        let paths = resolve_host_paths_from(
            Some(workflows_override.clone()),
            Some(runs_override.clone()),
            temp.path(),
            None,
        )
        .unwrap();

        assert_eq!(paths.workflows_dir, workflows_override);
        assert_eq!(paths.runs_dir, runs_override);
    }

    #[tokio::test]
    async fn failed_run_persists_metadata_and_returns_error() {
        let temp = tempdir().unwrap();
        let paths = HostPaths {
            workflows_dir: temp.path().join(DEFAULT_WORKFLOWS_DIR_NAME),
            runs_dir: temp.path().join(DEFAULT_RUNS_DIR_NAME),
        };
        let broken_package = DiscoveredWorkflowPackage {
            root_dir: temp.path().join("broken"),
            manifest_path: temp.path().join("broken").join("workflow.toml"),
            package: WorkflowPackage {
                id: "broken".to_string(),
                name: "Broken workflow".to_string(),
                summary: "summary".to_string(),
                entry: "start".to_string(),
                ui: WorkflowPackageUi {
                    input_artifact: "input".to_string(),
                    input_prompt: "Describe the request".to_string(),
                },
                nodes: vec![naaf_core::workflow_package::WorkflowPackageNode {
                    id: "start".to_string(),
                    kind: naaf_core::workflow_package::WorkflowNodeKind::Transformer,
                    step: "missing.step".to_string(),
                    config: serde_json::Value::Null,
                }],
                edges: vec![],
            },
        };

        let result = run_once(&paths, &broken_package, "fail").await;
        assert!(result.is_err());

        let run_dirs = std::fs::read_dir(&paths.runs_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(run_dirs.len(), 1);

        let record = load_run_record(&run_dirs[0]).unwrap();
        assert_eq!(record.workflow, "broken");
        assert_eq!(record.input, "fail");

        let state = StateStore::load(&run_dirs[0]).unwrap();
        assert_eq!(state.kind, StateKind::Proposed);
        assert_eq!(
            state
                .artifacts
                .get(&ArtifactKey::new("input"))
                .and_then(|value| value.as_text()),
            Some(&"fail".to_string())
        );

        let events =
            naaf_core::events::FilesystemEventStore::read_events(&run_dirs[0].join("events.log"))
                .unwrap();
        assert!(matches!(
            events.first(),
            Some(ExecutionEvent::RunStarted { .. })
        ));
        assert!(matches!(
            events.last(),
            Some(ExecutionEvent::RunFailed { .. })
        ));
    }

    #[tokio::test]
    async fn runtime_failure_persists_latest_state_snapshot() {
        let temp = tempdir().unwrap();
        let paths = HostPaths {
            workflows_dir: temp.path().join(DEFAULT_WORKFLOWS_DIR_NAME),
            runs_dir: temp.path().join(DEFAULT_RUNS_DIR_NAME),
        };
        let broken_package = DiscoveredWorkflowPackage {
            root_dir: temp.path().join("runtime-broken"),
            manifest_path: temp.path().join("runtime-broken").join("workflow.toml"),
            package: WorkflowPackage {
                id: "runtime-broken".to_string(),
                name: "Runtime broken workflow".to_string(),
                summary: "summary".to_string(),
                entry: "normalize".to_string(),
                ui: WorkflowPackageUi {
                    input_artifact: "input".to_string(),
                    input_prompt: "Describe the request".to_string(),
                },
                nodes: vec![naaf_core::workflow_package::WorkflowPackageNode {
                    id: "normalize".to_string(),
                    kind: naaf_core::workflow_package::WorkflowNodeKind::Transformer,
                    step: "openspec.normalize".to_string(),
                    config: serde_json::Value::Null,
                }],
                edges: vec![],
            },
        };

        let result = run_once(&paths, &broken_package, "fail later").await;
        assert!(result.is_err());

        let run_dirs = std::fs::read_dir(&paths.runs_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(run_dirs.len(), 1);

        let state = StateStore::load(&run_dirs[0]).unwrap();
        assert_eq!(state.kind, StateKind::Proposed);
        assert_eq!(
            state
                .artifacts
                .get(&ArtifactKey::new("input"))
                .and_then(|value| value.as_text()),
            Some(&"fail later".to_string())
        );

        let events =
            naaf_core::events::FilesystemEventStore::read_events(&run_dirs[0].join("events.log"))
                .unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ExecutionEvent::StepEntered { .. }))
        );
        assert!(matches!(
            events.last(),
            Some(ExecutionEvent::RunFailed { .. })
        ));
    }
}
