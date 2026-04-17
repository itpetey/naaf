use futures::future::LocalBoxFuture;
use naaf_core::{Materialiser, Task};
use uuid::Uuid;

use crate::client::{PointData, QdrantClient};
use crate::embedder::Embedder;
use crate::error::QdrantError;
use crate::payload::{KnowledgePayload, SearchResult};

pub struct QdrantSearch<R> {
    client: QdrantClient,
    embedder: Box<dyn Embedder>,
    top_k: usize,
    min_score: f32,
    repo: Option<String>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> QdrantSearch<R> {
    pub fn new(
        client: QdrantClient,
        embedder: Box<dyn Embedder>,
        top_k: usize,
        min_score: f32,
    ) -> Self {
        Self {
            client,
            embedder,
            top_k,
            min_score,
            repo: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }
}

impl<R: 'static> Task for QdrantSearch<R> {
    type Runtime = R;
    type Input = String;
    type Output = Vec<SearchResult>;
    type Error = QdrantError;

    fn run<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        query: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let vectors = self.embedder.embed(vec![query]).await?;
            let query_vector = vectors.into_iter().next().ok_or(QdrantError::NoResults)?;
            self.client
                .search(
                    query_vector,
                    self.top_k,
                    self.min_score,
                    self.repo.as_deref(),
                )
                .await
        })
    }
}

pub struct QdrantUpsert<R> {
    client: QdrantClient,
    embedder: Box<dyn Embedder>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> QdrantUpsert<R> {
    pub fn new(client: QdrantClient, embedder: Box<dyn Embedder>) -> Self {
        Self {
            client,
            embedder,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R: 'static> Materialiser for QdrantUpsert<R> {
    type Runtime = R;
    type Input = Vec<KnowledgePayload>;
    type Output = Vec<Uuid>;
    type Error = QdrantError;

    fn materialise<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        payloads: Self::Input,
    ) -> LocalBoxFuture<'a, Result<Self::Output, Self::Error>> {
        Box::pin(async move {
            let texts: Vec<String> = payloads.iter().map(|p| p.content.clone()).collect();
            let vectors = self.embedder.embed(texts).await?;
            let mut ids = Vec::with_capacity(payloads.len());
            let mut points = Vec::with_capacity(payloads.len());

            for (payload, vector) in payloads.into_iter().zip(vectors.into_iter()) {
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
        })
    }
}

pub struct QdrantSimilarityCheck<R> {
    client: QdrantClient,
    embedder: Box<dyn Embedder>,
    threshold: f32,
    _marker: std::marker::PhantomData<R>,
}

impl<R> QdrantSimilarityCheck<R> {
    pub fn new(client: QdrantClient, embedder: Box<dyn Embedder>, threshold: f32) -> Self {
        Self {
            client,
            embedder,
            threshold,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R: 'static> naaf_core::Check for QdrantSimilarityCheck<R> {
    type Runtime = R;
    type Subject = String;
    type Finding = SearchResult;
    type Error = QdrantError;

    fn check<'a>(
        &'a self,
        _runtime: &'a Self::Runtime,
        query: Self::Subject,
    ) -> LocalBoxFuture<'a, Result<Vec<Self::Finding>, Self::Error>> {
        Box::pin(async move {
            let vectors = self.embedder.embed(vec![query]).await?;
            let query_vector = vectors.into_iter().next().ok_or(QdrantError::NoResults)?;
            self.client
                .search(query_vector, 5, self.threshold, None)
                .await
        })
    }
}
