//! Workflow-aware TUI host for NAAF.

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::Parser;
use naaf_core::budget::{ExecCtx, LlmServiceConfig, ProviderType, Services};
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
    #[arg(long, env = "OPENAI_API_KEY", help = "OpenAI API key for LLM calls")]
    openai_api_key: Option<String>,
    #[arg(
        long,
        env = "OPENCODE_API_KEY",
        help = "OpenCode Go API key for LLM calls"
    )]
    opencode_api_key: Option<String>,
    #[arg(long, help = "LLM provider type (openai, opencode-go)")]
    provider: Option<String>,
    #[arg(long, help = "LLM model to use (e.g., gpt-5, glm-5)")]
    model: Option<String>,
    #[arg(long, help = "Custom LLM endpoint URL")]
    endpoint: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug)]
struct HostPaths {
    workflows_dir: PathBuf,
    runs_dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RecordedLlmConfig {
    provider: String,
    model: String,
    endpoint: Option<String>,
}

impl RecordedLlmConfig {
    fn from_resolved(config: &ResolvedLlmConfig) -> Self {
        Self {
            provider: provider_name(config.provider).to_string(),
            model: config.model.clone(),
            endpoint: config.endpoint.clone(),
        }
    }

    fn into_host_config(self, base: &HostLlmConfig) -> Result<HostLlmConfig> {
        Ok(HostLlmConfig {
            openai_api_key: base.openai_api_key.clone(),
            opencode_api_key: base.opencode_api_key.clone(),
            provider: Some(parse_provider_type(&self.provider)?),
            model: Some(self.model),
            endpoint: self.endpoint,
        })
    }
}

#[derive(Clone, Debug, Default)]
struct HostLlmConfig {
    openai_api_key: Option<String>,
    opencode_api_key: Option<String>,
    provider: Option<ProviderType>,
    model: Option<String>,
    endpoint: Option<String>,
}

#[derive(Clone, Debug)]
struct ResolvedLlmConfig {
    provider: ProviderType,
    model: String,
    endpoint: Option<String>,
    service_config: LlmServiceConfig,
}

impl HostLlmConfig {
    fn from_args(args: &Args) -> Result<Self> {
        Ok(Self {
            openai_api_key: args.openai_api_key.clone(),
            opencode_api_key: args.opencode_api_key.clone(),
            provider: args
                .provider
                .as_deref()
                .map(parse_provider_type)
                .transpose()?,
            model: args.model.clone(),
            endpoint: args.endpoint.clone(),
        })
    }

    fn service_config_for(&self, package: &DiscoveredWorkflowPackage) -> Result<ResolvedLlmConfig> {
        let runtime_llm = package.package.runtime.llm.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Workflow '{}' is missing runtime.llm configuration",
                package.package.id
            )
        })?;
        let allowed_providers = parse_allowed_providers(&runtime_llm.providers)?;
        let model = self.model.clone().or_else(|| {
            if runtime_llm.model.is_empty() || runtime_llm.model == "default" {
                None
            } else {
                Some(runtime_llm.model.clone())
            }
        });

        let provider = match self.provider {
            Some(provider) => provider,
            None => infer_provider_type(&allowed_providers, model.as_deref(), self)?,
        };

        if !allowed_providers.is_empty() && !allowed_providers.contains(&provider) {
            return Err(anyhow::anyhow!(
                "Workflow '{}' does not allow provider '{}'",
                package.package.id,
                provider_name(provider)
            ));
        }

        let model = match model {
            Some(model) => {
                validate_model_for_provider(provider, &model)?;
                model
            }
            None => default_model_for_provider(provider).to_string(),
        };

        let api_key = match provider {
            ProviderType::OpenAi => self.openai_api_key.clone(),
            ProviderType::OpenCodeGo => self.opencode_api_key.clone(),
        }
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No API key configured for provider '{}'. Use {} or the matching environment variable.",
                provider_name(provider),
                match provider {
                    ProviderType::OpenAi => "--openai-api-key",
                    ProviderType::OpenCodeGo => "--opencode-api-key",
                }
            )
        })?;

        let mut config = LlmServiceConfig::new()
            .provider(provider)
            .with_api_key(api_key);
        config = config.with_model(model.clone());
        if let Some(endpoint) = &self.endpoint {
            config = config.with_endpoint(endpoint.clone());
        }

        Ok(ResolvedLlmConfig {
            provider,
            model,
            endpoint: self.endpoint.clone(),
            service_config: config,
        })
    }
}

fn default_model_for_provider(provider: ProviderType) -> &'static str {
    match provider {
        ProviderType::OpenAi => "gpt-5",
        ProviderType::OpenCodeGo => "glm-5",
    }
}

fn provider_name(provider: ProviderType) -> &'static str {
    match provider {
        ProviderType::OpenAi => "openai",
        ProviderType::OpenCodeGo => "opencode-go",
    }
}

fn parse_allowed_providers(providers: &[String]) -> Result<Vec<ProviderType>> {
    providers
        .iter()
        .map(|provider| parse_provider_type(provider))
        .collect()
}

fn parse_provider_type(value: &str) -> Result<ProviderType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "openai" => Ok(ProviderType::OpenAi),
        "opencode-go" | "opencode_go" | "opencode" => Ok(ProviderType::OpenCodeGo),
        other => Err(anyhow::anyhow!(
            "Unsupported provider '{}'. Use 'openai' or 'opencode-go'",
            other
        )),
    }
}

fn infer_provider_type(
    allowed_providers: &[ProviderType],
    model: Option<&str>,
    config: &HostLlmConfig,
) -> Result<ProviderType> {
    if let Some(model) = model {
        let provider = provider_for_model(model)?;
        if !allowed_providers.is_empty() && !allowed_providers.contains(&provider) {
            return Err(anyhow::anyhow!(
                "Model '{}' is not compatible with the workflow's allowed providers",
                model
            ));
        }
        return Ok(provider);
    }

    let mut candidates = Vec::new();
    for provider in [ProviderType::OpenAi, ProviderType::OpenCodeGo] {
        if !allowed_providers.is_empty() && !allowed_providers.contains(&provider) {
            continue;
        }
        let configured = match provider {
            ProviderType::OpenAi => config.openai_api_key.is_some(),
            ProviderType::OpenCodeGo => config.opencode_api_key.is_some(),
        };
        if configured {
            candidates.push(provider);
        }
    }

    match candidates.as_slice() {
        [provider] => Ok(*provider),
        [] if allowed_providers.len() == 1 => Ok(allowed_providers[0]),
        [] => Err(anyhow::anyhow!(
            "Unable to determine LLM provider. Configure an API key, explicit provider, or workflow model."
        )),
        _ => Err(anyhow::anyhow!(
            "Multiple LLM providers are configured. Use --provider to select one explicitly."
        )),
    }
}

fn provider_for_model(model: &str) -> Result<ProviderType> {
    let model = model.trim().to_ascii_lowercase();
    if model.starts_with("gpt-") {
        return Ok(ProviderType::OpenAi);
    }
    if model.starts_with("glm-") || model.starts_with("kimi-") || model.starts_with("minimax-") {
        return Ok(ProviderType::OpenCodeGo);
    }

    Err(anyhow::anyhow!(
        "Unable to infer provider for model '{}'. Use --provider to specify one.",
        model
    ))
}

fn validate_model_for_provider(provider: ProviderType, model: &str) -> Result<()> {
    let is_supported = match provider {
        ProviderType::OpenAi => matches!(model, "gpt-5" | "gpt-54"),
        ProviderType::OpenCodeGo => {
            matches!(
                model,
                "glm-5" | "kimi-k2.5" | "minimax-m2.5" | "minimax-m2.7"
            )
        }
    };

    if !is_supported {
        return Err(anyhow::anyhow!(
            "Model '{}' is not supported by provider '{}'.",
            model,
            provider_name(provider)
        ));
    }

    Ok(())
}

#[derive(Parser, Debug)]
enum Command {
    #[command(about = "List discovered workflow packages")]
    Workflows,
    #[command(about = "Show details of a workflow package")]
    Show {
        #[arg(help = "Workflow package identifier")]
        workflow: String,
    },
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
    #[serde(default)]
    llm: Option<RecordedLlmConfig>,
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
    let llm_config = HostLlmConfig::from_args(&args)?;

    match args.command {
        Command::Workflows => list_workflows(&paths)?,
        Command::Show { workflow } => show_workflow(&paths, &workflow)?,
        Command::Run { workflow, input } => {
            run_workflow_host(&paths, &llm_config, &workflow, input, true).await?
        }
        Command::Runs => list_runs(&paths)?,
        Command::Inspect { run_id } => inspect_run(&paths, &run_id)?,
        Command::Replay { run_id } => replay_run(&paths, &llm_config, &run_id).await?,
    }

    Ok(())
}

async fn run_workflow_host(
    paths: &HostPaths,
    llm_config: &HostLlmConfig,
    workflow_id: &str,
    input: Option<String>,
    allow_clarification: bool,
) -> Result<()> {
    let packages = load_workflow_packages(paths)?;
    let package = find_workflow(&packages, workflow_id)?;

    if let Some(llm) = &package.package.runtime.llm {
        if llm.providers.is_empty() {
            println!(
                "Workflow '{}' is LLM-backed but has no declared providers.",
                workflow_id
            );
        } else {
            println!(
                "Workflow '{}' uses LLM provider(s): {}",
                workflow_id,
                llm.providers.join(", ")
            );
        }
        if !llm.model.is_empty() && llm.model != "default" {
            println!("Model: {}", llm.model);
        }
        println!();
    }

    let input = match input {
        Some(input) => input,
        None => prompt_for_input(&package.package.ui.input_prompt)?
            .ok_or_else(|| anyhow::anyhow!("No input provided"))?,
    };

    let interactive_clarification =
        allow_clarification && io::stdin().is_terminal() && io::stdout().is_terminal();
    let initial_run = run_once(paths, llm_config, package, &input).await?;
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
    let follow_up_run = run_once(paths, llm_config, package, &clarified_input).await?;

    println!("\nOriginal ambiguous run: {}", initial_run.run_id);
    println!("Clarified follow-up run: {}", follow_up_run.run_id);

    Ok(())
}

async fn run_once(
    paths: &HostPaths,
    llm_config: &HostLlmConfig,
    package: &DiscoveredWorkflowPackage,
    input: &str,
) -> Result<RunSummary> {
    let resolved = llm_config.service_config_for(package)?;
    let services = naaf_core::budget::LlmService::from_config(resolved.service_config.clone())?;
    run_once_with_services(
        paths,
        package,
        input,
        RecordedLlmConfig::from_resolved(&resolved),
        services,
    )
    .await
}

async fn run_once_with_services<S: Services + 'static>(
    paths: &HostPaths,
    package: &DiscoveredWorkflowPackage,
    input: &str,
    recorded_llm: RecordedLlmConfig,
    services: S,
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
            llm: Some(recorded_llm),
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

    let mut registry = WorkflowRegistry::<S>::new();
    naaf_openspec::register_legacy_steps(&mut registry);
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
    let mut ctx = ExecCtx::new(run_id, services)
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
        println!("    Entry: {}", package.package.entry);
        if let Some(llm) = &package.package.runtime.llm {
            let provider_str = if llm.providers.is_empty() {
                "default".to_string()
            } else {
                llm.providers.join(", ")
            };
            println!("    LLM: {}", provider_str);
        } else {
            println!("    LLM: invalid configuration");
        }
        if !package.package.ui.execution_guidance.is_empty() {
            println!("    Guidance: {}", package.package.ui.execution_guidance);
        }
        if !package.package.ui.primary_outputs.is_empty() {
            println!(
                "    Outputs: {}",
                package.package.ui.primary_outputs.join(", ")
            );
        }
        println!();
    }

    Ok(())
}

fn show_workflow(paths: &HostPaths, workflow_id: &str) -> Result<()> {
    let packages = load_workflow_packages(paths)?;
    let package = find_workflow(&packages, workflow_id)?;

    println!("Workflow: {}", package.package.name);
    println!("ID: {}", package.package.id);
    println!();
    println!("Summary: {}", package.package.summary);
    println!();

    println!("Entry: {}", package.package.entry);
    println!("Input artifact: {}", package.package.ui.input_artifact);
    println!("Input prompt: {}", package.package.ui.input_prompt);

    println!();
    println!("LLM Configuration:");
    if let Some(llm) = &package.package.runtime.llm {
        if !llm.providers.is_empty() {
            println!("  Providers: {}", llm.providers.join(", "));
        } else {
            println!("  Providers: default");
        }
    } else {
        println!("  Invalid or missing runtime.llm configuration");
    }

    if !package.package.ui.execution_guidance.is_empty() {
        println!();
        println!("Execution Guidance:");
        println!("  {}", package.package.ui.execution_guidance);
    }

    if !package.package.ui.primary_outputs.is_empty() {
        println!();
        println!("Primary Outputs:");
        for output in &package.package.ui.primary_outputs {
            println!("  - {}", output);
        }
    }

    if !package.package.runtime.inputs.is_empty() {
        println!();
        println!("Execution Inputs:");
        for input in &package.package.runtime.inputs {
            print!("  - {} ({})", input.id, input.artifact);
            if input.required {
                print!(" [required]");
            }
            println!();
            if !input.label.is_empty() {
                println!("    Label: {}", input.label);
            }
            if !input.prompt.is_empty() {
                println!("    Prompt: {}", input.prompt);
            }
        }
    }

    println!();
    println!("Nodes ({}):", package.package.nodes.len());
    for node in &package.package.nodes {
        println!("  - {} [{:?}] -> {}", node.id, node.kind, node.step);
    }

    println!();
    println!("Edges ({}):", package.package.edges.len());
    for edge in &package.package.edges {
        println!("  - {} -> {} [{:?}]", edge.from, edge.to, edge.kind);
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

async fn replay_run(paths: &HostPaths, llm_config: &HostLlmConfig, run_id: &str) -> Result<()> {
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

    let replay_llm_config = match record.llm {
        Some(recorded) => recorded.into_host_config(llm_config)?,
        None => llm_config.clone(),
    };

    run_workflow_host(
        paths,
        &replay_llm_config,
        &record.workflow,
        Some(record.input),
        false,
    )
    .await
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
    use naaf_core::workflow_package::{WorkflowPackage, WorkflowPackageRuntime, WorkflowPackageUi};
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
                runtime: WorkflowPackageRuntime {
                    llm: Some(naaf_core::workflow_package::WorkflowPackageLlmRuntime {
                        model: "gpt-5".to_string(),
                        providers: vec!["openai".to_string()],
                    }),
                    inputs: vec![],
                },
                ui: WorkflowPackageUi {
                    input_artifact: "input".to_string(),
                    input_prompt: "Describe the request".to_string(),
                    execution_guidance: String::new(),
                    primary_outputs: vec![],
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
            llm: Some(RecordedLlmConfig {
                provider: "openai".to_string(),
                model: "gpt-5".to_string(),
                endpoint: Some("https://example.invalid".to_string()),
            }),
        };

        save_run_record(temp.path(), &record).unwrap();
        let loaded = load_run_record(temp.path()).unwrap();
        assert_eq!(loaded.workflow, record.workflow);
        assert_eq!(loaded.input, record.input);
        assert_eq!(loaded.llm.unwrap().provider, "openai");
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
                llm: Some(RecordedLlmConfig {
                    provider: "openai".to_string(),
                    model: "gpt-5".to_string(),
                    endpoint: None,
                }),
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
        let llm_config = HostLlmConfig {
            openai_api_key: Some("test-key".to_string()),
            opencode_api_key: None,
            provider: Some(ProviderType::OpenAi),
            model: Some("gpt-5".to_string()),
            endpoint: Some("http://127.0.0.1:9".to_string()),
        };
        let broken_package = DiscoveredWorkflowPackage {
            root_dir: temp.path().join("broken"),
            manifest_path: temp.path().join("broken").join("workflow.toml"),
            package: WorkflowPackage {
                id: "broken".to_string(),
                name: "Broken workflow".to_string(),
                summary: "summary".to_string(),
                entry: "start".to_string(),
                runtime: WorkflowPackageRuntime {
                    llm: Some(naaf_core::workflow_package::WorkflowPackageLlmRuntime {
                        model: "gpt-5".to_string(),
                        providers: vec!["openai".to_string()],
                    }),
                    inputs: vec![],
                },
                ui: WorkflowPackageUi {
                    input_artifact: "input".to_string(),
                    input_prompt: "Describe the request".to_string(),
                    execution_guidance: String::new(),
                    primary_outputs: vec![],
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

        let result = run_once(&paths, &llm_config, &broken_package, "fail").await;
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
        let llm_config = HostLlmConfig {
            openai_api_key: Some("test-key".to_string()),
            opencode_api_key: None,
            provider: Some(ProviderType::OpenAi),
            model: Some("gpt-5".to_string()),
            endpoint: Some("http://127.0.0.1:9".to_string()),
        };
        let broken_package = DiscoveredWorkflowPackage {
            root_dir: temp.path().join("runtime-broken"),
            manifest_path: temp.path().join("runtime-broken").join("workflow.toml"),
            package: WorkflowPackage {
                id: "runtime-broken".to_string(),
                name: "Runtime broken workflow".to_string(),
                summary: "summary".to_string(),
                entry: "normalize".to_string(),
                runtime: WorkflowPackageRuntime {
                    llm: Some(naaf_core::workflow_package::WorkflowPackageLlmRuntime {
                        model: "gpt-5".to_string(),
                        providers: vec!["openai".to_string()],
                    }),
                    inputs: vec![],
                },
                ui: WorkflowPackageUi {
                    input_artifact: "input".to_string(),
                    input_prompt: "Describe the request".to_string(),
                    execution_guidance: String::new(),
                    primary_outputs: vec![],
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

        let result = run_once(&paths, &llm_config, &broken_package, "fail later").await;
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
