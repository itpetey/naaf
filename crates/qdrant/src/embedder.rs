use futures::future::LocalBoxFuture;

use crate::error::QdrantError;

pub trait Embedder {
    fn embed<'a>(
        &'a self,
        texts: Vec<String>,
    ) -> LocalBoxFuture<'a, Result<Vec<Vec<f32>>, QdrantError>>;

    fn dimension(&self) -> usize;
}

#[cfg(feature = "openai")]
pub mod openai {
    use futures::future::LocalBoxFuture;
    use reqwest::Client;
    use serde::Deserialize;

    use crate::error::QdrantError;

    use super::Embedder;

    #[derive(Debug, Clone, Deserialize)]
    struct EmbeddingResponse {
        data: Vec<EmbeddingData>,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct EmbeddingData {
        embedding: Vec<f32>,
    }

    pub struct OpenAiEmbedder {
        client: Client,
        api_key: String,
        model: String,
        base_url: String,
        dimension: usize,
    }

    impl OpenAiEmbedder {
        pub fn new(api_key: String) -> Self {
            Self::with_model(api_key, "text-embedding-3-small".to_string(), 1536)
        }

        pub fn with_model(api_key: String, model: String, dimension: usize) -> Self {
            Self {
                client: Client::new(),
                api_key,
                model,
                base_url: "https://api.openai.com/v1".to_string(),
                dimension,
            }
        }

        pub fn with_base_url(mut self, url: String) -> Self {
            self.base_url = url;
            self
        }
    }

    impl Embedder for OpenAiEmbedder {
        fn embed<'a>(
            &'a self,
            texts: Vec<String>,
        ) -> LocalBoxFuture<'a, Result<Vec<Vec<f32>>, QdrantError>> {
            Box::pin(async move {
                let response = self
                    .client
                    .post(format!("{}/embeddings", self.base_url))
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "model": self.model,
                        "input": texts,
                    }))
                    .send()
                    .await
                    .map_err(|e| QdrantError::Embedding(format!("request failed: {e}")))?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(QdrantError::Embedding(format!(
                        "embedding API returned {status}: {body}"
                    )));
                }

                let embedding_response: EmbeddingResponse = response.json().await.map_err(|e| {
                    QdrantError::Embedding(format!("failed to parse response: {e}"))
                })?;

                Ok(embedding_response
                    .data
                    .into_iter()
                    .map(|d| d.embedding)
                    .collect())
            })
        }

        fn dimension(&self) -> usize {
            self.dimension
        }
    }
}
