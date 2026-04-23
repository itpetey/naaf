//! Demonstrates using `KnowledgeLlmSessionBuilder` from `naaf-knowledge` in an LLM workflow.
//!
//! This example shows how to:
//! 1. Connect to Qdrant and set up an embedder
//! 2. Build a reusable knowledge-aware LLM session
//! 3. Use that session to create requests against indexed knowledge

use naaf_knowledge::{KnowledgeGroup, KnowledgeLlmSessionBuilder};
use naaf_llm::{Executor, ExecutorConfig, OpenAiClient, OpenAiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    println!("=== Knowledge Tool Example ===\n");

    let qdrant_url =
        std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
    let collection = std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "knowledge".to_string());
    let openai_api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    println!("Connecting to Qdrant at {}...", qdrant_url);
    let client = naaf_qdrant::QdrantClient::from_url(&qdrant_url, Option::<String>::None)?
        .with_collection(&collection);

    let embedder = naaf_qdrant::OpenAiEmbedder::new(openai_api_key.clone());

    println!("Creating knowledge LLM session...");
    let knowledge_group = KnowledgeGroup::new(
        collection.clone(),
        "Workspace knowledge",
        "Indexed project knowledge for retrieval-augmented answers",
    )
    .with_query_hints(["Search the knowledge base before answering project-specific questions"]);
    let knowledge = KnowledgeLlmSessionBuilder::new(Box::new(embedder))
        .with_system_prompt("You are a helpful assistant with access to a knowledge base.")
        .with_group(knowledge_group, client.clone())
        .with_search_defaults(5, 0.7)
        .build()
        .expect("failed to build knowledge session");

    let llm_config = OpenAiConfig::new(openai_api_key);
    let llm_client = OpenAiClient::new(llm_config);

    let executor = Executor::with_tools(llm_client, knowledge.tools().clone())
        .with_config(ExecutorConfig::new(5));

    println!("\n--- Query 1: What do you know about naaf-core? ---\n");

    let query1 = "What do you know about naaf-core?";
    let request = knowledge.request_with_user_message("gpt-4o", query1);

    let outcome = executor
        .execute(&(), request)
        .await
        .expect("execution failed");

    let final_msg = outcome.final_message();
    let content = final_msg.content.as_deref().unwrap_or("(no content)");
    println!("Answer: {}", content);

    println!("\n--- Query 2: Find info about Step and Retry ---\n");

    let query2 = "Find information about Step and Retry in the codebase";
    let request = knowledge.request_with_user_message("gpt-4o", query2);

    let outcome = executor
        .execute(&(), request)
        .await
        .expect("execution failed");

    let final_msg = outcome.final_message();
    let content = final_msg.content.as_deref().unwrap_or("(no content)");
    println!("Answer: {}", content);

    println!("\n=== Done ===");

    Ok(())
}
