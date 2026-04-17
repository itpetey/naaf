#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub qdrant: QdrantConfig,
    pub embedder: EmbedderConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub ingest: IngestConfig,
    #[serde(default)]
    pub query: QueryConfig,
}

#[derive(Debug, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    #[serde(default = "default_collection")]
    pub collection: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_collection() -> String {
    "knowledge".to_string()
}

#[derive(Debug, Deserialize)]
pub struct EmbedderConfig {
    #[serde(default = "default_embedder_provider")]
    pub provider: String,
    #[serde(default = "default_embedder_model")]
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_embedder_provider() -> String {
    "openai".to_string()
}

fn default_embedder_model() -> String {
    "text-embedding-3-small".to_string()
}

impl Default for EmbedderConfig {
    fn default() -> Self {
        Self {
            provider: default_embedder_provider(),
            model: default_embedder_model(),
            base_url: None,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_llm_provider() -> String {
    "openai".to_string()
}

fn default_llm_model() -> String {
    "gpt-4o-mini".to_string()
}

#[derive(Debug, Deserialize)]
pub struct IngestConfig {
    #[serde(default = "default_true")]
    pub extract_entities: bool,
    #[serde(default = "default_true")]
    pub extract_concepts: bool,
    #[serde(default = "default_true")]
    pub extract_comparisons: bool,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
    #[serde(default = "default_chunk_size")]
    pub max_chunk_size: usize,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            extract_entities: true,
            extract_concepts: true,
            extract_comparisons: true,
            chunk_overlap: 200,
            max_chunk_size: 1000,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_chunk_overlap() -> usize {
    200
}

fn default_chunk_size() -> usize {
    1000
}

#[derive(Debug, Deserialize)]
pub struct QueryConfig {
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_min_score")]
    pub min_score: f32,
    #[serde(default)]
    pub re_ingest_answers: bool,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            min_score: 0.7,
            re_ingest_answers: false,
        }
    }
}

fn default_top_k() -> usize {
    10
}

fn default_min_score() -> f32 {
    0.7
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = std::path::Path::new("knowledge.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default_config())
        }
    }

    fn default_config() -> Self {
        Self {
            qdrant: QdrantConfig {
                url: "http://localhost:6334".to_string(),
                collection: "knowledge".to_string(),
                api_key: None,
            },
            embedder: EmbedderConfig::default(),
            llm: LlmConfig::default(),
            ingest: IngestConfig::default(),
            query: QueryConfig::default(),
        }
    }
}