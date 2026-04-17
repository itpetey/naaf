use qdrant_client::qdrant::Filter;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, Distance, PointId, PointStruct, QueryPointsBuilder,
    RetrievedPoint, ScoredPoint, ScrollPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
    point_id::PointIdOptions,
};
use qdrant_client::{Payload, Qdrant};
use uuid::Uuid;

use crate::chunker::SourceInfo;
use crate::embedder::Embedder;
use crate::error::QdrantError;
use crate::payload::{EntityType, KnowledgePayload, SearchResult, SourceType};

fn scored_point_to_search(point: ScoredPoint, min_score: f32) -> Option<SearchResult> {
    if point.score < min_score {
        return None;
    }
    let id = point.id.and_then(|id| match id.point_id_options? {
        PointIdOptions::Num(n) => Some(Uuid::from_u128(n as u128)),
        PointIdOptions::Uuid(s) => s.parse().ok(),
    })?;
    let payload: Payload = point.payload.into();
    let payload: KnowledgePayload = serde_json::from_value(serde_json::Value::from(payload))
        .unwrap_or_else(|_| {
            KnowledgePayload::new(String::new(), String::new(), EntityType::Source)
        });
    Some(SearchResult {
        id,
        score: point.score,
        payload,
    })
}

fn retrieved_point_to_search(point: RetrievedPoint) -> Option<SearchResult> {
    let id = point.id.and_then(|id| match id.point_id_options? {
        PointIdOptions::Num(n) => Some(Uuid::from_u128(n as u128)),
        PointIdOptions::Uuid(s) => s.parse().ok(),
    })?;
    let payload: Payload = point.payload.into();
    let payload: KnowledgePayload = serde_json::from_value(serde_json::Value::from(payload))
        .unwrap_or_else(|_| {
            KnowledgePayload::new(String::new(), String::new(), EntityType::Source)
        });
    Some(SearchResult {
        id,
        score: 1.0,
        payload,
    })
}

#[derive(Clone)]
pub struct QdrantClient {
    inner: Qdrant,
    collection: String,
    url: String,
}

impl QdrantClient {
    pub fn from_url(url: &str) -> Result<Self, QdrantError> {
        let inner = Qdrant::from_url(url)
            .build()
            .map_err(|e| QdrantError::Client(e.to_string()))?;
        Ok(Self {
            inner,
            collection: "knowledge".to_string(),
            url: url.to_string(),
        })
    }

    pub fn with_api_key(self, api_key: &str) -> Result<Self, QdrantError> {
        let inner = Qdrant::from_url(&self.url)
            .api_key(api_key)
            .build()
            .map_err(|e| QdrantError::Client(e.to_string()))?;
        Ok(Self {
            inner,
            collection: self.collection,
            url: self.url,
        })
    }

    pub fn with_collection(mut self, collection: &str) -> Self {
        self.collection = collection.to_string();
        self
    }

    pub fn inner(&self) -> &Qdrant {
        &self.inner
    }

    pub fn collection(&self) -> &str {
        &self.collection
    }

    pub async fn create_collection_if_not_exists(
        &self,
        dimension: usize,
    ) -> Result<(), QdrantError> {
        let exists = self
            .inner
            .collection_exists(&self.collection)
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        if !exists {
            self.inner
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection).vectors_config(
                        VectorParamsBuilder::new(dimension as u64, Distance::Cosine),
                    ),
                )
                .await
                .map_err(|e| QdrantError::Client(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn upsert_points(&self, points: Vec<PointData>) -> Result<(), QdrantError> {
        let qdrant_points: Vec<PointStruct> = points
            .iter()
            .map(|p| {
                let payload: Payload =
                    Payload::try_from(serde_json::to_value(&p.payload).unwrap_or_default())
                        .unwrap_or_default();
                PointStruct::new(p.id.to_string(), p.vector.clone(), payload)
            })
            .collect();

        self.inner
            .upsert_points(UpsertPointsBuilder::new(&self.collection, qdrant_points).wait(true))
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        Ok(())
    }

    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        min_score: f32,
        repo: Option<&str>,
    ) -> Result<Vec<SearchResult>, QdrantError> {
        let mut builder = QueryPointsBuilder::new(&self.collection)
            .query(query_vector)
            .limit(limit as u64)
            .with_payload(true);

        if let Some(repo_name) = repo {
            let filter = Filter::must([Condition::matches("repo", repo_name.to_string())]);
            builder = builder.filter(filter);
        }

        let response = self
            .inner
            .query(builder)
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        let results: Vec<SearchResult> = response
            .result
            .into_iter()
            .filter_map(|p| scored_point_to_search(p, min_score))
            .collect();

        Ok(results)
    }

    pub async fn scroll(
        &self,
        limit: usize,
        offset: Option<&str>,
    ) -> Result<Vec<SearchResult>, QdrantError> {
        let mut builder = ScrollPointsBuilder::new(&self.collection)
            .limit(limit as u32)
            .with_payload(true);

        if let Some(offset_id) = offset {
            builder = builder.offset(PointId::from(offset_id.to_string()));
        }

        let response = self
            .inner
            .scroll(builder)
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        let results: Vec<SearchResult> = response
            .result
            .into_iter()
            .filter_map(retrieved_point_to_search)
            .collect();

        Ok(results)
    }
}

pub struct PointData {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub payload: KnowledgePayload,
}

pub struct QdrantAgent<R> {
    client: QdrantClient,
    embedder: Box<dyn Embedder>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> QdrantAgent<R> {
    pub fn new(client: QdrantClient, embedder: Box<dyn Embedder>) -> Self {
        Self {
            client,
            embedder,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn client(&self) -> &QdrantClient {
        &self.client
    }

    pub fn embedder(&self) -> &dyn Embedder {
        self.embedder.as_ref()
    }

    pub async fn init_collection(&self) -> Result<(), QdrantError> {
        let dimension = self.embedder.dimension();
        self.client.create_collection_if_not_exists(dimension).await
    }

    pub async fn search(
        &self,
        query: &str,
        top_k: usize,
        min_score: f32,
        repo: Option<&str>,
    ) -> Result<Vec<SearchResult>, QdrantError> {
        let vectors = self.embedder.embed(vec![query.to_string()]).await?;
        let query_vector = vectors.into_iter().next().ok_or(QdrantError::NoResults)?;
        self.client
            .search(query_vector, top_k, min_score, repo)
            .await
    }

    pub async fn upsert_chunks(
        &self,
        chunks: Vec<crate::chunker::Chunk>,
        source_info: &SourceInfo,
        repo: Option<&str>,
    ) -> Result<Vec<Uuid>, QdrantError> {
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let vectors = self.embedder.embed(texts).await?;

        let mut ids = Vec::with_capacity(chunks.len());
        let mut points = Vec::with_capacity(chunks.len());

        for (chunk, vector) in chunks.iter().zip(vectors.iter()) {
            let id = Uuid::new_v4();
            let entity_type = match chunk.metadata.source_type {
                SourceType::Markdown => EntityType::Source,
                SourceType::Code => EntityType::Source,
                SourceType::Conversation => EntityType::Source,
                SourceType::Paper => EntityType::Source,
                SourceType::PlainText => EntityType::Source,
            };

            let mut payload = KnowledgePayload::new(
                chunk
                    .metadata
                    .heading
                    .clone()
                    .unwrap_or_else(|| format!("chunk-{}", chunk.metadata.index)),
                chunk.text.clone(),
                entity_type,
            );

            if let Some(repo_name) = repo {
                payload = payload.with_repo(repo_name);
            }

            if let Some(ref path) = chunk.metadata.path {
                payload.source_metadata = Some(crate::payload::SourceMetadata {
                    source_type: chunk.metadata.source_type.clone(),
                    path: Some(std::path::PathBuf::from(path)),
                    title: source_info.title.clone(),
                    language: chunk.metadata.language.clone(),
                    line_range: Some((chunk.metadata.start_char, chunk.metadata.end_char)),
                });
            }

            payload.tags = chunk
                .metadata
                .heading
                .clone()
                .map(|h| vec![h])
                .unwrap_or_default();

            points.push(PointData {
                id,
                vector: vector.clone(),
                payload,
            });
            ids.push(id);
        }

        self.client.upsert_points(points).await?;
        Ok(ids)
    }

    pub async fn upsert_knowledge(
        &self,
        entries: Vec<(KnowledgePayload, Vec<f32>)>,
    ) -> Result<Vec<Uuid>, QdrantError> {
        let mut ids = Vec::with_capacity(entries.len());
        let mut points = Vec::with_capacity(entries.len());

        for (payload, vector) in entries {
            let id = Uuid::new_v4();
            points.push(PointData {
                id,
                vector,
                payload,
            });
            ids.push(id);
        }

        self.client.upsert_points(points).await?;
        Ok(ids)
    }
}
