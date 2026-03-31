//! CLI entry point.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use naaf_model::ModelProvider;
use naaf_openspec::{Phase, openspec_happy_path};
use naaf_orchestrator::{
    artifact::{Artifact, ArtifactId, ArtifactKind},
    journal::{Event, Journal},
    run::{Outcome, Run, RunId, TaskId, TerminalReason},
    store::ArtifactStore,
    workflow::{DefaultExecutionEngine, run_workflow},
};

const RUNS_DIR: &str = ".runs";

#[derive(Parser, Debug)]
#[command(name = "naaf", about = "NAAF - Not Another AI Framework CLI", version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    #[command(about = "Run a workflow with a prompt")]
    Run {
        #[arg(help = "The prompt to process")]
        prompt: String,
        #[arg(
            short,
            long,
            value_enum,
            default_value = "openai",
            help = "Model provider"
        )]
        provider: ProviderChoice,
        #[arg(short, long, help = "Model identifier")]
        model: Option<String>,
    },
    #[command(about = "List all runs")]
    List,
    #[command(about = "Inspect a run")]
    Inspect {
        #[arg(help = "Run ID to inspect")]
        run_id: String,
    },
    #[command(about = "View artifacts from a run")]
    Artifacts {
        #[arg(help = "Run ID to view artifacts for")]
        run_id: String,
        #[arg(short, long, help = "Artifact ID to view")]
        view: Option<String>,
        #[arg(short, long, help = "Output as JSON")]
        json: bool,
    },
    #[command(about = "View journal events for a run")]
    Journal {
        #[arg(help = "Run ID to view journal for")]
        run_id: String,
        #[arg(short, long, help = "Comma-separated event types to filter")]
        filter: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ProviderChoice {
    Openai,
    OpencodeGo,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Run {
            prompt,
            provider,
            model,
        } => {
            run(prompt, provider, model).await?;
        }
        Command::List => {
            list_runs()?;
        }
        Command::Inspect { run_id } => {
            inspect_run(&run_id)?;
        }
        Command::Artifacts { run_id, view, json } => {
            artifacts(&run_id, view.as_deref(), json)?;
        }
        Command::Journal { run_id, filter } => {
            journal(&run_id, filter.as_deref())?;
        }
    }

    Ok(())
}

async fn run(prompt: String, provider_choice: ProviderChoice, model: Option<String>) -> Result<()> {
    match provider_choice {
        ProviderChoice::Openai => {
            let api_key = std::env::var("OPENAI_API_KEY").context(
                "OPENAI_API_KEY environment variable not set.\n\
                 Set it with: export OPENAI_API_KEY=your-api-key",
            )?;
            let model = model.unwrap_or_else(|| "gpt-5".to_string());
            let provider = naaf_providers::openai::OpenAiModel::gpt5(api_key);
            execute_workflow(prompt, Arc::new(provider), model).await
        }
        ProviderChoice::OpencodeGo => {
            let api_key = std::env::var("OPENCODE_GO_API_KEY").context(
                "OPENCODE_GO_API_KEY environment variable not set.\n\
                 Set it with: export OPENCODE_GO_API_KEY=your-api-key",
            )?;
            let model_input = model.unwrap_or_else(|| "glm-5".to_string());
            let model_str = model_input.clone();
            match model_input.to_lowercase().as_str() {
                "glm-5" | "glm5" => {
                    let provider = naaf_providers::opencode_go::OpenCodeGoModel::glm5(api_key);
                    execute_workflow(prompt, Arc::new(provider), model_str).await
                }
                "kimi-k2.5" | "kimi-k25" | "kimik25" => {
                    let provider = naaf_providers::opencode_go::OpenCodeGoModel::kimik25(api_key);
                    execute_workflow(prompt, Arc::new(provider), model_str).await
                }
                "minimax-m2.5" | "minimaxm25" => {
                    let provider =
                        naaf_providers::opencode_go::OpenCodeGoModel::minimaxm25(api_key);
                    execute_workflow(prompt, Arc::new(provider), model_str).await
                }
                "minimax-m2.7" | "minimaxm27" => {
                    let provider =
                        naaf_providers::opencode_go::OpenCodeGoModel::minimaxm27(api_key);
                    execute_workflow(prompt, Arc::new(provider), model_str).await
                }
                _ => {
                    anyhow::bail!(
                        "Unknown OpenCode Go model: {}\n\
                         Available models: glm-5, kimi-k2.5, minimax-m2.5, minimax-m2.7",
                        model_input
                    )
                }
            }
        }
    }
}

async fn execute_workflow<P: ModelProvider + Sync + 'static>(
    prompt: String,
    provider: Arc<P>,
    model: String,
) -> Result<()> {
    let runs_dir = PathBuf::from(RUNS_DIR);
    std::fs::create_dir_all(&runs_dir)?;

    let task_id = TaskId::new();
    let run_id = RunId::new();
    let worktree = runs_dir.join(run_id.0.to_string());
    std::fs::create_dir_all(&worktree)?;

    let store = ArtifactStore::new(&runs_dir)?;
    let journal = Journal::new(&runs_dir)?;

    let user_prompt_artifact = Artifact::new(
        run_id,
        ArtifactKind::UserPrompt,
        vec![],
        worktree.join("user_prompt.bin"),
    );
    store.save(&user_prompt_artifact, prompt.as_bytes())?;

    println!("Running workflow...");
    println!("Run ID: {}", run_id);
    println!("Model: {}", model);

    let engine = DefaultExecutionEngine::new(provider, model, store, journal);

    let workflow = openspec_happy_path();

    let mut run = Run::new(task_id, worktree);

    let outcome = run_workflow(&engine, &workflow, &mut run).await?;

    match &outcome {
        Outcome::Done => {
            println!("\nOutcome: SUCCESS");
        }
        Outcome::Failed(reason) => {
            println!("\nOutcome: FAILED");
            if let TerminalReason::Failed { message } = reason {
                println!("Reason: {}", message);
            }
        }
        Outcome::Escalated(reason) => {
            println!("\nOutcome: ESCALATED");
            if let TerminalReason::Escalated { message } = reason {
                println!("Reason: {}", message);
            }
        }
        Outcome::InProgress => {
            println!("\nOutcome: IN PROGRESS");
        }
    }

    println!("\nArtifacts saved to: {}/{}", RUNS_DIR, run.id);

    Ok(())
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
        "{:<38} {:<15} {:<12} TIMESTAMP",
        "RUN ID", "PHASE", "OUTCOME"
    );
    println!("{}", "-".repeat(80));

    for entry in runs {
        let run_id_str = entry.file_name().to_string_lossy().into_owned();
        let run_dir = entry.path();

        let (phase, outcome, timestamp) = if let Ok(run_uuid) = run_id_str.parse::<uuid::Uuid>() {
            let run_uuid = RunId(run_uuid);
            let journal = match Journal::new(&run_dir) {
                Ok(j) => j,
                Err(_) => {
                    continue;
                }
            };

            let mut last_phase = "unknown".to_string();
            let mut last_outcome = "unknown".to_string();
            let mut last_timestamp: Option<DateTime<Utc>> = None;

            if let Ok(events) = journal.for_run(run_uuid) {
                for event in events.flatten() {
                    match &event {
                        Event::RunCreated { timestamp, .. } => {
                            last_timestamp = Some(*timestamp);
                        }
                        Event::RunStarted {
                            run_id: _,
                            timestamp,
                        } => {
                            last_phase = format!("{:?}", Phase::default());
                            last_timestamp = Some(*timestamp);
                        }
                        Event::TransitionExecuted {
                            to_phase,
                            timestamp,
                            ..
                        } => {
                            last_phase = format!("{:?}", to_phase);
                            last_timestamp = Some(*timestamp);
                        }
                        Event::RunCompleted { timestamp, .. } => {
                            last_outcome = "done".to_string();
                            last_timestamp = Some(*timestamp);
                        }
                        Event::RunFailed {
                            reason: _,
                            timestamp,
                            ..
                        } => {
                            last_outcome = "failed".to_string();
                            last_timestamp = Some(*timestamp);
                        }
                        Event::RunEscalated {
                            reason: _,
                            timestamp,
                            ..
                        } => {
                            last_outcome = "escalated".to_string();
                            last_timestamp = Some(*timestamp);
                        }
                        _ => {}
                    }
                }
            }
            (last_phase, last_outcome, last_timestamp)
        } else {
            continue;
        };

        let timestamp_str = timestamp
            .map(|t: DateTime<Utc>| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<38} {:<15} {:<12} {}",
            run_id_str, phase, outcome, timestamp_str
        );
    }

    Ok(())
}

fn inspect_run(run_id: &str) -> Result<()> {
    let run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;
    let run_uuid = RunId(run_uuid);
    let run_dir = PathBuf::from(RUNS_DIR).join(run_id);
    if !run_dir.exists() {
        eprintln!("Run not found: {}", run_id);
        std::process::exit(1);
    }

    println!("Run: {}", run_id);
    println!("Directory: {}", run_dir.display());

    let journal = Journal::new(&run_dir)?;

    let mut phase = "unknown".to_string();
    let mut outcome = "unknown".to_string();

    if let Ok(events) = journal.for_run(run_uuid) {
        for event in events.flatten() {
            match &event {
                Event::RunStarted {
                    timestamp: _,
                    run_id: _,
                } => {
                    phase = format!("{:?}", Phase::default());
                }
                Event::TransitionExecuted {
                    to_phase,
                    from_phase: _,
                    run_id: _,
                    timestamp: _,
                    worker_id: _,
                    artifact_id: _,
                } => {
                    phase = format!("{:?}", to_phase);
                }
                Event::RunCompleted { .. } => {
                    outcome = "done".to_string();
                }
                Event::RunFailed { .. } => {
                    outcome = "failed".to_string();
                }
                Event::RunEscalated { .. } => {
                    outcome = "escalated".to_string();
                }
                _ => {}
            }
        }
    }

    println!("Phase: {}", phase);
    println!("Outcome: {}", outcome);

    Ok(())
}

fn artifacts(run_id: &str, view_artifact: Option<&str>, json: bool) -> Result<()> {
    let run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;
    let run_uuid = RunId(run_uuid);
    let run_dir = PathBuf::from(RUNS_DIR).join(run_id);
    if !run_dir.exists() {
        eprintln!("Run not found: {}", run_id);
        std::process::exit(1);
    }

    let store = ArtifactStore::new(&run_dir)?;
    let metadata = store.list_metadata(run_uuid)?;

    if metadata.is_empty() {
        println!("No artifacts found");
        return Ok(());
    }

    if json {
        let output: Vec<_> = metadata
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id.0,
                    "kind": m.kind.name(),
                    "created_at": m.created_at.to_rfc3339()
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if let Some(artifact_id_str) = view_artifact {
        if let Ok(id) = artifact_id_str.parse::<uuid::Uuid>() {
            let artifact_id = ArtifactId(id);
            match store.load(artifact_id, run_uuid) {
                Ok((_artifact, content)) => {
                    println!("{}", String::from_utf8_lossy(&content));
                }
                Err(e) => {
                    eprintln!("Error loading artifact: {}", e);
                }
            }
        } else {
            eprintln!("Invalid artifact ID format");
        }
        return Ok(());
    }

    println!("{:<38} {:<25} CREATED AT", "ID", "KIND");
    println!("{}", "-".repeat(80));

    for m in &metadata {
        println!(
            "{:<38} {:<25} {}",
            m.id.0,
            m.kind.name(),
            m.created_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    Ok(())
}

fn journal(run_id: &str, filter: Option<&str>) -> Result<()> {
    let run_uuid: uuid::Uuid = run_id.parse().context("Invalid run ID")?;
    let run_uuid = RunId(run_uuid);
    let run_dir = PathBuf::from(RUNS_DIR).join(run_id);
    if !run_dir.exists() {
        eprintln!("Run not found: {}", run_id);
        std::process::exit(1);
    }

    let journal = Journal::new(&run_dir)?;

    let events: Vec<_> = journal.for_run(run_uuid)?.filter_map(|e| e.ok()).collect();

    if events.is_empty() {
        println!("No journal events found");
        return Ok(());
    }

    let filter_types: Option<Vec<&str>> = filter.map(|f| f.split(',').map(|s| s.trim()).collect());

    for event in events {
        let timestamp = match &event {
            Event::TaskCreated { timestamp, .. } => timestamp,
            Event::RunCreated { timestamp, .. } => timestamp,
            Event::RunStarted { timestamp, .. } => timestamp,
            Event::ReviewStarted { timestamp, .. } => timestamp,
            Event::TransitionExecuted { timestamp, .. } => timestamp,
            Event::ArtifactCreated { timestamp, .. } => timestamp,
            Event::FindingCreated { timestamp, .. } => timestamp,
            Event::FindingResolved { timestamp, .. } => timestamp,
            Event::RunCompleted { timestamp, .. } => timestamp,
            Event::RunFailed { timestamp, .. } => timestamp,
            Event::RunEscalated { timestamp, .. } => timestamp,
        };
        let event_type = match &event {
            Event::TaskCreated { .. } => "task_created",
            Event::RunCreated { .. } => "run_created",
            Event::RunStarted { .. } => "run_started",
            Event::ReviewStarted { .. } => "review_started",
            Event::TransitionExecuted { .. } => "transition_executed",
            Event::ArtifactCreated { .. } => "artifact_created",
            Event::FindingCreated { .. } => "finding_created",
            Event::FindingResolved { .. } => "finding_resolved",
            Event::RunCompleted { .. } => "run_completed",
            Event::RunFailed { .. } => "run_failed",
            Event::RunEscalated { .. } => "run_escalated",
        };

        if let Some(ref types) = filter_types
            && !types.contains(&event_type)
        {
            continue;
        }

        println!("{}  {}", timestamp.format("%Y-%m-%d %H:%M:%S"), event_type);
    }

    Ok(())
}
