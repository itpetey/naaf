use clap::{Parser, Subcommand};
use tracing::info;

mod config;

#[derive(Parser)]
#[command(name = "naaf", about = "Knowledge base management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Knowledge base commands
    #[command(subcommand)]
    Kb(KbCommands),
}

#[derive(Subcommand)]
enum KbCommands {
    /// Initialise the knowledge base (create Qdrant collection)
    Init,
    /// Ingest a file or directory into the knowledge base
    Ingest {
        /// Path to file or directory to ingest
        path: String,
        /// Repository name for filtering
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Query the knowledge base
    Query {
        /// Natural language query
        query: Vec<String>,
        /// Repository name for filtering
        #[arg(short, long)]
        repo: Option<String>,
    },
    /// Lint the knowledge base for issues
    Lint,
    /// List entries in the knowledge base
    List {
        /// Maximum number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let config = config::Config::load().expect("failed to load config");
    let cli = Cli::parse();

    match cli.command {
        Commands::Kb(cmd) => match cmd {
            KbCommands::Init => {
                info!("Connecting to Qdrant at {}", config.qdrant.url);
                let mut client = naaf_qdrant::QdrantClient::from_url(&config.qdrant.url)
                    .expect("failed to connect");
                if let Some(ref api_key) = config.qdrant.api_key {
                    client = client.with_api_key(api_key).expect("failed to set API key");
                }
                client = client.with_collection(&config.qdrant.collection);

                let embedder = make_embedder(&config);
                let agent = naaf_qdrant::QdrantAgent::<()>::new(client, embedder);

                agent
                    .init_collection()
                    .await
                    .expect("failed to create collection");

                println!("Collection '{}' is ready.", config.qdrant.collection);
            }
            KbCommands::Ingest { path, repo } => {
                let path = std::path::Path::new(&path);
                if !path.exists() {
                    eprintln!("Path does not exist: {}", path.display());
                    std::process::exit(1);
                }

                let mut client = naaf_qdrant::QdrantClient::from_url(&config.qdrant.url)
                    .expect("failed to connect");
                if let Some(ref api_key) = config.qdrant.api_key {
                    client = client.with_api_key(api_key).expect("failed to set API key");
                }
                client = client.with_collection(&config.qdrant.collection);

                let embedder = make_embedder(&config);
                let agent = naaf_qdrant::QdrantAgent::<()>::new(client, embedder);

                if path.is_dir() {
                    info!("Walking directory: {}", path.display());
                    let report =
                        naaf_knowledge::ingest::ingest_directory(&agent, path, repo.as_deref())
                            .await
                            .expect("directory ingestion failed");

                    for (file_path, result) in &report.reports {
                        match result {
                            Ok(r) => {
                                println!("  {} — {} chunks", file_path.display(), r.chunks_count)
                            }
                            Err(e) => eprintln!("  {} — ERROR: {}", file_path.display(), e),
                        }
                    }

                    println!();
                    println!(
                        "Ingested {} files, skipped {} errors",
                        report.files_ingested, report.files_skipped
                    );
                    println!(
                        "Total: {} chunks, {} source entries",
                        report.total_chunks,
                        report.total_source_ids.len()
                    );
                } else {
                    info!("Ingesting: {}", path.display());

                    let report = naaf_knowledge::ingest::ingest_file(&agent, path, repo.as_deref())
                        .await
                        .expect("ingestion failed");

                    println!("Ingested {} chunks", report.chunks_count);
                    println!("Source IDs: {:?}", report.source_ids);
                }
            }
            KbCommands::Query { query, repo } => {
                let query_text = query.join(" ");

                let mut client = naaf_qdrant::QdrantClient::from_url(&config.qdrant.url)
                    .expect("failed to connect");
                if let Some(ref api_key) = config.qdrant.api_key {
                    client = client.with_api_key(api_key).expect("failed to set API key");
                }
                client = client.with_collection(&config.qdrant.collection);

                let embedder = make_embedder(&config);
                let agent = naaf_qdrant::QdrantAgent::<()>::new(client, embedder);

                let results = naaf_knowledge::query::query_knowledge(
                    &agent,
                    &query_text,
                    config.query.top_k,
                    config.query.min_score,
                    repo.as_deref(),
                )
                .await
                .expect("query failed");

                if results.is_empty() {
                    println!("No results found.");
                } else {
                    for result in &results {
                        println!(
                            "[{}] {} (score: {:.3})",
                            result.payload.entity_type, result.payload.title, result.score
                        );
                        println!("  {}", truncate(&result.payload.content, 200));
                        println!();
                    }
                    println!("{} results found.", results.len());
                }
            }
            KbCommands::Lint => {
                let mut client = naaf_qdrant::QdrantClient::from_url(&config.qdrant.url)
                    .expect("failed to connect");
                if let Some(ref api_key) = config.qdrant.api_key {
                    client = client.with_api_key(api_key).expect("failed to set API key");
                }
                client = client.with_collection(&config.qdrant.collection);

                let report = naaf_knowledge::lint::lint_collection(&client)
                    .await
                    .expect("lint failed");

                println!("Scanned {} entries", report.entries_scanned);
                if report.issues.is_empty() {
                    println!("No issues found.");
                } else {
                    println!("{} issues found:", report.issues.len());
                    for issue in &report.issues {
                        println!("  [{:?}] {}", issue.issue_type, issue.description);
                        if let Some(ref suggestion) = issue.suggestion {
                            println!("    Suggestion: {suggestion}");
                        }
                    }
                }
            }
            KbCommands::List { limit } => {
                let mut client = naaf_qdrant::QdrantClient::from_url(&config.qdrant.url)
                    .expect("failed to connect");
                if let Some(ref api_key) = config.qdrant.api_key {
                    client = client.with_api_key(api_key).expect("failed to set API key");
                }
                client = client.with_collection(&config.qdrant.collection);

                let entries = client.scroll(limit, None).await.expect("scroll failed");

                if entries.is_empty() {
                    println!("No entries found.");
                } else {
                    for entry in &entries {
                        println!(
                            "[{}] {} (created: {})",
                            entry.payload.entity_type,
                            entry.payload.title,
                            entry.payload.created_at
                        );
                        println!("  {}", truncate(&entry.payload.content, 100));
                    }
                    println!("{} entries shown.", entries.len());
                }
            }
        },
    }
}

fn make_embedder(config: &config::Config) -> Box<dyn naaf_qdrant::Embedder> {
    let base_url = config.embedder.base_url.clone();
    match config.embedder.provider.as_str() {
        "lm_studio" => {
            let model = config.embedder.model.clone();
            let dimension = 768;
            let mut embedder =
                naaf_qdrant::OpenAiEmbedder::with_model(String::new(), model, dimension);
            embedder = embedder.with_base_url(base_url.unwrap_or_else(|| {
                config::EmbedderConfig::lm_studio_default_base_url().to_string()
            }));
            Box::new(embedder)
        }
        _ => {
            let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
            let model = config.embedder.model.clone();
            let dimension = if model.contains("text-embedding-3-large") {
                3072
            } else {
                1536
            };
            let mut embedder = naaf_qdrant::OpenAiEmbedder::with_model(api_key, model, dimension);
            if let Some(url) = base_url {
                embedder = embedder.with_base_url(url);
            }
            Box::new(embedder)
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}...")
    }
}
