mod config;

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "knowledge", about = "Knowledge base management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise the knowledge base (create Qdrant collection)
    Init,
    /// Ingest a file or directory into the knowledge base
    Ingest {
        /// Path to file or directory to ingest
        path: String,
    },
    /// Query the knowledge base
    Query {
        /// Natural language query
        query: Vec<String>,
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

    let cli = Cli::parse();
    let config = config::Config::load().expect("failed to load config");

    match cli.command {
        Commands::Init => {
            info!("Connecting to Qdrant at {}", config.qdrant.url);
            let mut client =
                naaf_qdrant::QdrantClient::from_url(&config.qdrant.url).expect("failed to connect");
            if let Some(ref api_key) = config.qdrant.api_key {
                client = client.with_api_key(api_key).expect("failed to set API key");
            }
            client = client.with_collection(&config.qdrant.collection);

            let embedder = naaf_qdrant::OpenAiEmbedder::new(
                std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
            );

            let agent = naaf_qdrant::QdrantAgent::<()>::new(client, Box::new(embedder));

            agent
                .init_collection()
                .await
                .expect("failed to create collection");

            println!("Collection '{}' is ready.", config.qdrant.collection);
        }
        Commands::Ingest { path } => {
            let path = std::path::Path::new(&path);
            if !path.exists() {
                eprintln!("Path does not exist: {}", path.display());
                std::process::exit(1);
            }

            let mut client =
                naaf_qdrant::QdrantClient::from_url(&config.qdrant.url).expect("failed to connect");
            if let Some(ref api_key) = config.qdrant.api_key {
                client = client.with_api_key(api_key).expect("failed to set API key");
            }
            client = client.with_collection(&config.qdrant.collection);

            let embedder = naaf_qdrant::OpenAiEmbedder::new(
                std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
            );

            let agent = naaf_qdrant::QdrantAgent::<()>::new(client, Box::new(embedder));

            if path.is_dir() {
                info!("Walking directory: {}", path.display());
                let report = naaf_knowledge::ingest::ingest_directory(&agent, path)
                    .await
                    .expect("directory ingestion failed");

                for (file_path, result) in &report.reports {
                    match result {
                        Ok(r) => println!(
                            "  {} — {} chunks",
                            file_path.display(),
                            r.chunks_count
                        ),
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

                let report = naaf_knowledge::ingest::ingest_file(&agent, path)
                    .await
                    .expect("ingestion failed");

                println!("Ingested {} chunks", report.chunks_count);
                println!("Source IDs: {:?}", report.source_ids);
            }
        }
        Commands::Query { query } => {
            let query_text = query.join(" ");

            let mut client =
                naaf_qdrant::QdrantClient::from_url(&config.qdrant.url).expect("failed to connect");
            if let Some(ref api_key) = config.qdrant.api_key {
                client = client.with_api_key(api_key).expect("failed to set API key");
            }
            client = client.with_collection(&config.qdrant.collection);

            let embedder = naaf_qdrant::OpenAiEmbedder::new(
                std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
            );

            let agent = naaf_qdrant::QdrantAgent::<()>::new(client, Box::new(embedder));

            let results = naaf_knowledge::query::query_knowledge(
                &agent,
                &query_text,
                config.query.top_k,
                config.query.min_score,
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
        Commands::Lint => {
            let mut client =
                naaf_qdrant::QdrantClient::from_url(&config.qdrant.url).expect("failed to connect");
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
        Commands::List { limit } => {
            let mut client =
                naaf_qdrant::QdrantClient::from_url(&config.qdrant.url).expect("failed to connect");
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
                        entry.payload.entity_type, entry.payload.title, entry.payload.created_at
                    );
                    println!("  {}", truncate(&entry.payload.content, 100));
                }
                println!("{} entries shown.", entries.len());
            }
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
