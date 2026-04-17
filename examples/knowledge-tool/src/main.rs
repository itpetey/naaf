//! Demonstrates using `KnowledgeTool` from `naaf-knowledge` in an LLM workflow.
//!
//! This example shows how to:
//! 1. Connect to Qdrant and set up an embedder
//! 2. Register `KnowledgeTool` in a tool registry
//! 3. Use the tool with an LLM agent to answer questions about indexed knowledge

use naaf_knowledge::KnowledgeTool;
use naaf_llm::{
    CompletionRequest, Executor, ExecutorConfig, Message, OpenAiClient, OpenAiConfig, ToolRegistry,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    println!("=== Knowledge Tool Example ===\n");

    let qdrant_url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
    let collection = std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "knowledge".to_string());
    let openai_api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set");

    println!("Connecting to Qdrant at {}...", qdrant_url);
    let mut client = naaf_qdrant::QdrantClient::from_url(&qdrant_url)?;
    client = client.with_collection(&collection);

    let embedder = naaf_qdrant::OpenAiEmbedder::new(openai_api_key.clone());

    println!("Creating KnowledgeTool...");
    let knowledge_tool = KnowledgeTool::new(
        client.clone(),
        Box::new(embedder),
        5,  // top_k
        0.7, // min_score
    );

    let mut registry = ToolRegistry::new();
    registry
        .register(knowledge_tool)
        .expect("failed to register knowledge tool");

    let llm_config = OpenAiConfig::new(openai_api_key);
    let llm_client = OpenAiClient::new(llm_config);

    let executor = Executor::with_tools(llm_client, registry)
        .with_config(ExecutorConfig::new(5));

    println!("\n--- Query 1: What do you know about naaf-core? ---\n");

    let query1 = "What do you know about naaf-core?";
    let messages = vec![
        Message::system("You are a helpful assistant with access to a knowledge base. \
            Use the knowledge_search tool to find relevant information before answering."),
        Message::user(query1),
    ];

    let request = CompletionRequest::new(
        "gpt-4o".to_string(),
        messages,
    );

    let outcome = executor
        .execute(&(), request)
        .await
        .expect("execution failed");

    let final_msg = outcome.final_message();
    let content = final_msg.content.as_deref().unwrap_or("(no content)");
    println!("Answer: {}", content);

    println!("\n--- Query 2: Find info about Step and Retry ---\n");

    let query2 = "Find information about Step and Retry in the codebase";
    let messages = vec![
        Message::user(query2),
    ];

    let request = CompletionRequest::new(
        "gpt-4o".to_string(),
        messages,
    );

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