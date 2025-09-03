// Database module - Shared database abstraction for service integration
//
// This module provides a unified Database struct that implements the DatabaseAccess trait,
// allowing it to be used across CLI, HTTP API, and MCP interfaces while maintaining
// a consistent API surface.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::{
    create_binary_trigram_index, create_file_storage, create_primary_index, create_trigram_index,
    create_wrapped_storage,
    services::{AnalysisServiceDatabase, DatabaseAccess},
    Index, Storage, ValidatedDocumentId,
};

/// Main database abstraction that coordinates storage and indices
///
/// This struct serves as the primary interface to KotaDB's storage and indexing systems,
/// implementing the DatabaseAccess trait required by all services.
pub struct Database {
    pub storage: Arc<Mutex<dyn Storage>>,
    pub primary_index: Arc<Mutex<dyn Index>>,
    pub trigram_index: Arc<Mutex<dyn Index>>,
    // Cache for path -> document ID lookups (built lazily)
    pub path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl Database {
    /// Create a new Database instance with storage and indices
    ///
    /// # Arguments
    /// * `db_path` - Root path for database storage
    /// * `use_binary_index` - Whether to use binary or text-based trigram index
    pub async fn new(db_path: &Path, use_binary_index: bool) -> Result<Self> {
        let storage_path = db_path.join("storage");
        let primary_index_path = db_path.join("primary_index");
        let trigram_index_path = db_path.join("trigram_index");

        // Create directories if they don't exist
        std::fs::create_dir_all(&storage_path)?;
        std::fs::create_dir_all(&primary_index_path)?;
        std::fs::create_dir_all(&trigram_index_path)?;

        let storage = create_file_storage(
            storage_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid storage path: {:?}", storage_path))?,
            Some(100), // Cache size
        )
        .await?;

        let primary_index = create_primary_index(
            primary_index_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid primary index path: {:?}", primary_index_path)
            })?,
            Some(1000), // Cache size
        )
        .await?;

        // Choose trigram index implementation based on binary flag
        let trigram_index_arc: Arc<Mutex<dyn Index>> = if use_binary_index {
            Arc::new(Mutex::new(
                create_binary_trigram_index(
                    trigram_index_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                    })?,
                    Some(1000), // Cache size
                )
                .await?,
            ))
        } else {
            Arc::new(Mutex::new(
                create_trigram_index(
                    trigram_index_path.to_str().ok_or_else(|| {
                        anyhow::anyhow!("Invalid trigram index path: {:?}", trigram_index_path)
                    })?,
                    Some(1000), // Cache size
                )
                .await?,
            ))
        };

        // Apply wrappers for production safety
        let wrapped_storage = create_wrapped_storage(storage, 100).await;

        Ok(Self {
            storage: Arc::new(Mutex::new(wrapped_storage)),
            primary_index: Arc::new(Mutex::new(primary_index)),
            trigram_index: trigram_index_arc,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get database statistics (document count and total size)
    pub async fn stats(&self) -> Result<(usize, usize)> {
        let all_docs = self.storage.lock().await.list_all().await?;
        let doc_count = all_docs.len();
        let total_size: usize = all_docs.iter().map(|d| d.size).sum();
        Ok((doc_count, total_size))
    }
}

// Implement DatabaseAccess trait for the Database struct
impl DatabaseAccess for Database {
    fn storage(&self) -> Arc<Mutex<dyn Storage>> {
        self.storage.clone()
    }

    fn primary_index(&self) -> Arc<Mutex<dyn Index>> {
        self.primary_index.clone()
    }

    fn trigram_index(&self) -> Arc<Mutex<dyn Index>> {
        self.trigram_index.clone()
    }

    fn path_cache(&self) -> Arc<RwLock<HashMap<String, ValidatedDocumentId>>> {
        self.path_cache.clone()
    }
}

// Implement AnalysisServiceDatabase trait for the Database struct
impl AnalysisServiceDatabase for Database {
    fn storage(&self) -> Arc<Mutex<dyn Storage>> {
        self.storage.clone()
    }
}
