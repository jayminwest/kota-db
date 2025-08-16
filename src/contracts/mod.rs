// Contracts Module - Stage 2: Contract-First Design
// Defines all contracts and interfaces for KotaDB components

pub mod connection_pool;
pub mod optimization;
pub mod performance;

// Re-export key types for convenience
pub use performance::{
    ComplexityClass, ComplexityContract, MemoryContract, PerformanceGuarantee,
    PerformanceMeasurement,
};

pub use connection_pool::{
    ConnectionMetrics, ConnectionPool, ConnectionPoolConfig, ConnectionStats, RateLimitResult,
    RateLimiter, ResourceMonitor,
};

pub use optimization::{
    BalanceInfo, BulkOperationResult, BulkOperationType, BulkOperations, ConcurrentAccess,
    ContentionMetrics, MemoryOptimization, MemoryUsage, OptimizationSLA, SLAComplianceReport,
    TreeAnalysis, TreeStructureMetrics,
};

// Core domain contracts (re-exported from original contracts.rs)
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{
    ValidatedDocumentId, ValidatedLimit, ValidatedPageId, ValidatedPath, ValidatedSearchQuery,
    ValidatedTag, ValidatedTitle,
};

/// Core storage contract
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Open a storage instance at the given path
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized;

    /// Insert a new document
    async fn insert(&mut self, document: Document) -> Result<()>;

    /// Get a document by ID
    async fn get(&self, id: &ValidatedDocumentId) -> Result<Option<Document>>;

    /// Update an existing document
    async fn update(&mut self, document: Document) -> Result<()>;

    /// Delete a document by ID
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool>;

    /// List all documents
    async fn list_all(&self) -> Result<Vec<Document>>;

    /// Sync changes to persistent storage
    async fn sync(&mut self) -> Result<()>;

    /// Flush any pending changes
    async fn flush(&mut self) -> Result<()>;

    /// Close the storage instance
    async fn close(self) -> Result<()>;
}

/// Core index contract
#[async_trait::async_trait]
pub trait Index: Send + Sync {
    /// Open an index instance at the given path
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized;

    /// Insert a key-value pair into the index
    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()>;

    /// Insert with document content for content-aware indices
    ///
    /// Default implementation calls insert() for backward compatibility.
    /// Indices that need content (like trigram) should override this method.
    async fn insert_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        _content: &[u8],
    ) -> Result<()> {
        // Default implementation ignores content and delegates to insert()
        self.insert(id, path).await
    }

    /// Update an existing entry in the index
    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()>;

    /// Update with document content for content-aware indices
    ///
    /// Default implementation calls update() for backward compatibility.
    /// Indices that need content (like trigram) should override this method.
    async fn update_with_content(
        &mut self,
        id: ValidatedDocumentId,
        path: ValidatedPath,
        _content: &[u8],
    ) -> Result<()> {
        // Default implementation ignores content and delegates to update()
        self.update(id, path).await
    }

    /// Delete an entry from the index
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool>;

    /// Search the index with a query
    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>>;

    /// Sync changes to persistent storage
    async fn sync(&mut self) -> Result<()>;

    /// Flush any pending changes
    async fn flush(&mut self) -> Result<()>;

    /// Close the index instance
    async fn close(self) -> Result<()>;
}

/// Document representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: ValidatedDocumentId,
    pub path: ValidatedPath,
    pub title: ValidatedTitle,
    pub content: Vec<u8>,
    pub tags: Vec<ValidatedTag>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub size: usize,
    /// Optional vector embedding for semantic search (typically 1536 dimensions for OpenAI)
    pub embedding: Option<Vec<f32>>,
}

impl Document {
    /// Create a new document
    pub fn new(
        id: ValidatedDocumentId,
        path: ValidatedPath,
        title: ValidatedTitle,
        content: Vec<u8>,
        tags: Vec<ValidatedTag>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        let size = content.len();
        Self {
            id,
            path,
            title,
            content,
            tags,
            created_at,
            updated_at,
            size,
            embedding: None,
        }
    }

    /// Create a new document with embedding
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_embedding(
        id: ValidatedDocumentId,
        path: ValidatedPath,
        title: ValidatedTitle,
        content: Vec<u8>,
        tags: Vec<ValidatedTag>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        embedding: Vec<f32>,
    ) -> Self {
        let size = content.len();
        Self {
            id,
            path,
            title,
            content,
            tags,
            created_at,
            updated_at,
            size,
            embedding: Some(embedding),
        }
    }
}

/// Query representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub search_terms: Vec<ValidatedSearchQuery>,
    pub tags: Vec<ValidatedTag>,
    pub path_pattern: Option<String>,
    pub limit: ValidatedLimit,
    pub offset: ValidatedPageId,
}

impl Query {
    /// Create a query with specific parameters (for backward compatibility)
    pub fn new(
        text: Option<String>,
        _tags: Option<Vec<String>>,
        _path_pattern: Option<String>,
        limit: usize,
    ) -> anyhow::Result<Self> {
        let mut search_terms = Vec::new();
        if let Some(text) = text {
            if !text.is_empty() && text != "*" {
                search_terms.push(ValidatedSearchQuery::new(&text, 1)?); // min_length = 1
            }
        }

        Ok(Self {
            search_terms,
            tags: Vec::new(),
            path_pattern: None,
            limit: ValidatedLimit::new(limit, 1000)?,
            offset: ValidatedPageId::new(1)?,
        })
    }

    /// Create an empty query with defaults
    pub fn empty() -> Self {
        Self {
            search_terms: Vec::new(),
            tags: Vec::new(),
            path_pattern: None,
            limit: ValidatedLimit::new(10, 1000).expect("Default limit values are valid"),
            offset: ValidatedPageId::new(1).expect("Default page ID is valid"),
        }
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::empty()
    }
}

/// Storage metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageMetrics {
    pub total_documents: u64,
    pub total_size_bytes: u64,
    pub avg_document_size: f64,
    pub storage_efficiency: f64,
    pub fragmentation: f64,
}

/// Page identifier for pagination
pub type PageId = ValidatedPageId;

/// Transaction interface for ACID operations
pub trait Transaction {
    #[allow(async_fn_in_trait)]
    async fn commit(&mut self) -> Result<()>;
    #[allow(async_fn_in_trait)]
    async fn rollback(&mut self) -> Result<()>;
    fn is_active(&self) -> bool;
}

/// Metrics collection interface
pub trait MetricsCollector {
    fn record_operation(&self, operation: &str, duration: std::time::Duration);
    fn record_size(&self, metric: &str, size: u64);
    fn get_metrics(&self) -> HashMap<String, f64>;
}

/// Health check interface
pub trait HealthCheck {
    #[allow(async_fn_in_trait)]
    async fn health(&self) -> Result<HealthStatus>;
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// Configuration interface
pub trait Configuration {
    fn get_config(&self) -> &DatabaseConfig;
    fn update_config(&mut self, config: DatabaseConfig) -> Result<()>;
}

/// Database configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub storage_path: PathBuf,
    pub max_file_size: u64,
    pub cache_size: usize,
    pub sync_interval: std::time::Duration,
    pub enable_compression: bool,
    pub enable_encryption: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./data"),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            cache_size: 1000,
            sync_interval: std::time::Duration::from_secs(5),
            enable_compression: false,
            enable_encryption: false,
        }
    }
}
