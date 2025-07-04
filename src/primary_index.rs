// Primary Index Implementation - Stage 2: Contract-First Design
// This implements the Index trait using a file-based B+ tree structure
// Designed to work with all Stage 6 component library wrappers

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::contracts::{Index, Query};
use crate::pure::{btree, extract_all_pairs};
use crate::types::{ValidatedDocumentId, ValidatedPath};
use crate::validation;
use crate::wrappers::MeteredIndex;

/// Primary index implementation using file-based B+ tree
///
/// This is the basic index engine that implements the Index trait.
/// It should always be used with the Stage 6 MeteredIndex wrapper for production use.
pub struct PrimaryIndex {
    /// Root directory for the index
    index_path: PathBuf,
    /// B+ tree for O(log n) operations (Document ID -> Path)
    btree_root: RwLock<btree::BTreeRoot>,
    /// Write-ahead log for durability
    wal_writer: RwLock<Option<tokio::fs::File>>,
    /// Index metadata
    metadata: RwLock<IndexMetadata>,
}

/// Metadata for the primary index
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexMetadata {
    version: u32,
    document_count: usize,
    created: i64,
    updated: i64,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            version: 1,
            document_count: 0,
            created: now,
            updated: now,
        }
    }
}

impl PrimaryIndex {
    /// Create directory structure for the index
    async fn ensure_directories(&self) -> Result<()> {
        let paths = [
            self.index_path.join("data"),
            self.index_path.join("wal"),
            self.index_path.join("meta"),
        ];

        for path in &paths {
            fs::create_dir_all(path)
                .await
                .with_context(|| format!("Failed to create index directory: {}", path.display()))?;
        }

        Ok(())
    }

    /// Initialize write-ahead log
    async fn init_wal(&self) -> Result<()> {
        let wal_path = self.index_path.join("wal").join("current.wal");
        let wal_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .with_context(|| format!("Failed to open WAL file: {}", wal_path.display()))?;

        *self.wal_writer.write().await = Some(wal_file);
        Ok(())
    }

    /// Load existing index from disk
    async fn load_existing_index(&self) -> Result<()> {
        let data_dir = self.index_path.join("data");

        if !data_dir.exists() {
            return Ok(());
        }

        // Load metadata
        let metadata_path = self.index_path.join("meta").join("metadata.json");
        if metadata_path.exists() {
            let metadata_content = fs::read_to_string(&metadata_path)
                .await
                .with_context(|| format!("Failed to read metadata: {}", metadata_path.display()))?;

            let metadata: IndexMetadata = serde_json::from_str(&metadata_content)
                .context("Failed to deserialize index metadata")?;

            *self.metadata.write().await = metadata;
        }

        // Load B+ tree data
        let btree_path = data_dir.join("btree_data.json");
        if btree_path.exists() {
            let btree_content = fs::read_to_string(&btree_path).await.with_context(|| {
                format!("Failed to read B+ tree data: {}", btree_path.display())
            })?;

            // For now, rebuild tree from key-value pairs (future optimization: serialize tree structure)
            let raw_mappings: HashMap<String, String> = serde_json::from_str(&btree_content)
                .context("Failed to deserialize B+ tree data")?;

            // Rebuild B+ tree from stored key-value pairs
            let mut btree_root = btree::create_empty_tree();
            for (id_str, path_str) in raw_mappings {
                let uuid = Uuid::parse_str(&id_str)
                    .with_context(|| format!("Invalid UUID in B+ tree data: {}", id_str))?;

                let doc_id = ValidatedDocumentId::from_uuid(uuid)
                    .with_context(|| format!("Invalid document ID: {}", id_str))?;

                let validated_path = ValidatedPath::new(&path_str)
                    .with_context(|| format!("Invalid path in B+ tree data: {}", path_str))?;

                btree_root = btree::insert_into_tree(btree_root, doc_id, validated_path)
                    .with_context(|| {
                        format!("Failed to insert into B+ tree: {} -> {}", id_str, path_str)
                    })?;
            }

            *self.btree_root.write().await = btree_root;
        }

        Ok(())
    }

    /// Save metadata to disk
    async fn save_metadata(&self) -> Result<()> {
        let metadata_path = self.index_path.join("meta").join("metadata.json");
        let metadata = self.metadata.read().await;

        let content =
            serde_json::to_string_pretty(&*metadata).context("Failed to serialize metadata")?;

        fs::write(&metadata_path, content)
            .await
            .with_context(|| format!("Failed to write metadata: {}", metadata_path.display()))?;

        Ok(())
    }

    /// Save B+ tree data to disk
    async fn save_mappings(&self) -> Result<()> {
        let btree_path = self.index_path.join("data").join("btree_data.json");
        let btree_root = self.btree_root.read().await;

        // Extract all key-value pairs from B+ tree for serialization
        // Future optimization: serialize the tree structure directly
        let all_pairs = extract_all_pairs(&btree_root)?;

        // Convert to serializable format
        let raw_mappings: HashMap<String, String> = all_pairs
            .iter()
            .map(|(doc_id, path)| (doc_id.as_uuid().to_string(), path.to_string()))
            .collect();

        let content = serde_json::to_string_pretty(&raw_mappings)
            .context("Failed to serialize B+ tree data")?;

        fs::write(&btree_path, content)
            .await
            .with_context(|| format!("Failed to write B+ tree data: {}", btree_path.display()))?;

        Ok(())
    }

    /// Update metadata counters
    async fn update_metadata(&self, document_count_delta: i32) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        if document_count_delta < 0 {
            let decrease = (-document_count_delta) as usize;
            if metadata.document_count < decrease {
                bail!(
                    "Document count would go negative: {} - {}",
                    metadata.document_count,
                    decrease
                );
            }
            metadata.document_count -= decrease;
        } else {
            metadata.document_count += document_count_delta as usize;
        }

        metadata.updated = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Validate preconditions for insert operation
    fn validate_insert_preconditions(
        key: &ValidatedDocumentId,
        value: &ValidatedPath,
    ) -> Result<()> {
        // Key validation
        let uuid = key.as_uuid();
        if uuid.is_nil() {
            bail!("Insert precondition failed: Key cannot be nil UUID");
        }

        // Value validation - ValidatedPath already ensures non-empty and valid format
        // Additional checks can be added here if needed

        Ok(())
    }

    /// Validate preconditions for delete operation
    fn validate_delete_preconditions(key: &ValidatedDocumentId) -> Result<()> {
        let uuid = key.as_uuid();
        if uuid.is_nil() {
            bail!("Delete precondition failed: Key cannot be nil UUID");
        }

        Ok(())
    }

    /// Validate preconditions for search operation
    fn validate_search_preconditions(query: &Query) -> Result<()> {
        // Query validation is handled by Query::new() constructor
        // Additional index-specific validation can be added here

        Ok(())
    }

    /// Validate postcondition that entry is searchable after insert
    async fn validate_insert_postcondition(
        &self,
        key: &ValidatedDocumentId,
        value: &ValidatedPath,
    ) -> Result<()> {
        let btree_root = self.btree_root.read().await;

        match btree::search_in_tree(&btree_root, key) {
            Some(stored_path) => {
                if stored_path != *value {
                    bail!("Insert postcondition failed: Stored path {} does not match inserted path {}", 
                          stored_path, value);
                }
                Ok(())
            }
            None => {
                bail!(
                    "Insert postcondition failed: Key {} not found after insertion",
                    key.as_uuid()
                );
            }
        }
    }

    /// Validate postcondition that key is not searchable after delete
    async fn validate_delete_postcondition(&self, key: &ValidatedDocumentId) -> Result<()> {
        let btree_root = self.btree_root.read().await;

        if btree::search_in_tree(&btree_root, key).is_some() {
            bail!(
                "Delete postcondition failed: Key {} still exists after deletion",
                key.as_uuid()
            );
        }

        Ok(())
    }
}

#[async_trait]
impl Index for PrimaryIndex {
    /// Open an index instance at the given path
    async fn open(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        // Validate path using existing validation
        validation::path::validate_directory_path(path)?;

        let index_path = PathBuf::from(path);
        let mut index = Self {
            index_path: index_path.clone(),
            btree_root: RwLock::new(btree::create_empty_tree()),
            wal_writer: RwLock::new(None),
            metadata: RwLock::new(IndexMetadata::default()),
        };

        // Ensure directory structure exists
        index.ensure_directories().await?;

        // Initialize WAL
        index.init_wal().await?;

        // Load existing state from disk
        index
            .load_existing_index()
            .await
            .context("Failed to load existing index from disk")?;

        Ok(index)
    }

    /// Insert a key-value pair into the primary index
    ///
    /// # Contract
    /// - Preconditions: Key must be non-nil, Value must be valid path
    /// - Postconditions: Entry is searchable immediately, previous value overwritten
    /// - Invariants: Document count increases by 1 (if new key)
    async fn insert(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // Stage 2: Contract enforcement - validate preconditions
        Self::validate_insert_preconditions(&id, &path)?;

        let was_new_key;

        // Check if key exists before insertion (for metadata counting)
        {
            let btree_root = self.btree_root.read().await;
            was_new_key = btree::search_in_tree(&btree_root, &id).is_none();
        }

        // Insert into B+ tree using pure functions (O(log n))
        {
            let mut btree_root = self.btree_root.write().await;
            *btree_root = btree::insert_into_tree(btree_root.clone(), id.clone(), path.clone())
                .context("Failed to insert into B+ tree")?;
        }

        // Update metadata
        if was_new_key {
            self.update_metadata(1).await?;
        } else {
            // Update timestamp even for overwrites
            let mut metadata = self.metadata.write().await;
            metadata.updated = chrono::Utc::now().timestamp();
        }

        // Stage 2: Contract enforcement - validate postconditions
        self.validate_insert_postcondition(&id, &path)
            .await
            .context("Insert postcondition validation failed")?;

        Ok(())
    }

    /// Remove a key from the primary index
    ///
    /// # Contract  
    /// - Preconditions: Key must be valid format
    /// - Postconditions: Key no longer appears in searches, space reclaimed
    /// - Invariants: Document count decreases by 1 (if key existed)
    async fn delete(&mut self, id: &ValidatedDocumentId) -> Result<bool> {
        // Stage 2: Contract enforcement - validate preconditions
        Self::validate_delete_preconditions(id)?;

        let existed;

        // Check if key exists before deletion
        {
            let btree_root = self.btree_root.read().await;
            existed = btree::search_in_tree(&btree_root, id).is_some();
        }

        if existed {
            // Use O(log n) B+ tree deletion algorithm
            let mut btree_root = self.btree_root.write().await;

            *btree_root = btree::delete_from_tree(btree_root.clone(), id)
                .context("Failed to delete from B+ tree")?;

            // Update metadata
            self.update_metadata(-1).await?;
        }

        // Stage 2: Contract enforcement - validate postconditions
        self.validate_delete_postcondition(id)
            .await
            .context("Delete postcondition validation failed")?;

        Ok(existed)
    }

    /// Search the primary index
    ///
    /// # Contract
    /// - Preconditions: Query must be valid for index type
    /// - Postconditions: Results sorted by relevance, all matches returned
    /// - Invariants: Does not modify index state
    ///
    /// # Note
    /// Primary index only supports wildcard searches (no text search terms).
    /// For text search, use a dedicated text search index.
    async fn search(&self, query: &Query) -> Result<Vec<ValidatedDocumentId>> {
        // Stage 2: Contract enforcement - validate preconditions
        Self::validate_search_preconditions(query)?;

        // Primary index only supports wildcard ("*") searches
        // If there are specific search terms, return empty (no text search capability)
        if !query.search_terms.is_empty() {
            return Ok(Vec::new());
        }

        let btree_root = self.btree_root.read().await;

        // Use B+ tree traversal to get all documents (O(n) for full scan, but sorted)
        let all_pairs = extract_all_pairs(&btree_root)?;

        // Extract just the document IDs for results
        let mut results: Vec<ValidatedDocumentId> =
            all_pairs.into_iter().map(|(doc_id, _)| doc_id).collect();

        // Apply limit from query
        let limit_value = query.limit.get();
        if results.len() > limit_value {
            results.truncate(limit_value);
        }

        // Results are already sorted by key order from B+ tree

        Ok(results)
    }

    /// Flush index to disk
    ///
    /// # Contract
    /// - Preconditions: Index must be valid
    /// - Postconditions: All changes persisted, index recoverable after crash
    /// - Invariants: Index state unchanged
    async fn flush(&mut self) -> Result<()> {
        // Save all persistent state
        self.save_metadata()
            .await
            .context("Failed to save metadata during flush")?;

        self.save_mappings()
            .await
            .context("Failed to save mappings during flush")?;

        // Sync WAL
        if let Some(wal_file) = self.wal_writer.write().await.as_mut() {
            wal_file
                .sync_all()
                .await
                .context("Failed to sync WAL during flush")?;
        }

        Ok(())
    }

    /// Update an existing entry in the index
    async fn update(&mut self, id: ValidatedDocumentId, path: ValidatedPath) -> Result<()> {
        // For B+ tree, update is the same as insert (it overwrites)
        self.insert(id, path).await
    }

    /// Sync changes to persistent storage
    async fn sync(&mut self) -> Result<()> {
        // Similar to flush for this implementation
        self.flush().await
    }

    /// Close the index instance
    async fn close(self) -> Result<()> {
        // Save final state
        // Note: We need to work around the fact that close() consumes self
        // but save methods require &self

        // For this simple implementation, we just drop the WAL writer
        // In a real implementation, we'd properly close all resources

        drop(self.wal_writer);
        Ok(())
    }
}

/// Create a fully wrapped PrimaryIndex with all Stage 6 components
///
/// This is the recommended way to create a production-ready primary index.
/// It automatically applies Stage 6 MeteredIndex wrapper for metrics and observability.
pub async fn create_primary_index(
    path: &str,
    _cache_capacity: Option<usize>,
) -> Result<MeteredIndex<PrimaryIndex>> {
    // Stage 2: Validate path using existing validation
    validation::path::validate_directory_path(path)?;

    let index_path = PathBuf::from(path);
    let index = PrimaryIndex {
        index_path,
        btree_root: RwLock::new(btree::create_empty_tree()),
        wal_writer: RwLock::new(None),
        metadata: RwLock::new(IndexMetadata::default()),
    };

    // Ensure directory structure exists
    index.ensure_directories().await?;

    // Initialize WAL
    index.init_wal().await?;

    // Load existing data
    index.load_existing_index().await?;

    // Apply Stage 6 wrapper for automatic metrics
    Ok(MeteredIndex::new(index, "primary".to_string()))
}

/// Alternative factory function for testing without cache parameter
/// Used internally by tests that don't need to specify cache capacity
pub async fn create_primary_index_for_tests(path: &str) -> Result<PrimaryIndex> {
    validation::path::validate_directory_path(path)?;

    let index_path = PathBuf::from(path);
    let index = PrimaryIndex {
        index_path,
        btree_root: RwLock::new(btree::create_empty_tree()),
        wal_writer: RwLock::new(None),
        metadata: RwLock::new(IndexMetadata::default()),
    };

    index.ensure_directories().await?;
    index.init_wal().await?;
    index.load_existing_index().await?;

    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_primary_index_contract_enforcement() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("contract_test");

        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        // Test precondition validation
        let valid_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let valid_path = ValidatedPath::new("/test/contract.md")?;

        // This should succeed
        index.insert(valid_id.clone(), valid_path.clone()).await?;

        // Test postcondition - document should be findable
        let query = Query::new(Some("*".to_string()), None, None, 10)?;
        let results = index.search(&query).await?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], valid_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_primary_index_metadata_management() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let index_path = temp_dir.path().join("metadata_test");

        let mut index = create_primary_index_for_tests(index_path.to_str().unwrap()).await?;

        // Check initial metadata
        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 0);
            assert_eq!(metadata.version, 1);
        }

        // Insert document and check metadata update
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let doc_path = ValidatedPath::new("/test/metadata.md")?;

        index.insert(doc_id.clone(), doc_path).await?;

        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 1);
        }

        // Delete document and check metadata update
        index.delete(&doc_id).await?;

        {
            let metadata = index.metadata.read().await;
            assert_eq!(metadata.document_count, 0);
        }

        Ok(())
    }
}
