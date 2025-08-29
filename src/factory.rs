//! Factory functions for creating production-ready components
//!
//! This module provides factory functions that return fully-wrapped components
//! with all production features enabled (tracing, validation, retries, caching).

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::contracts::Storage;
use crate::file_storage::create_file_storage;
use crate::graph_storage::GraphStorageConfig;
use crate::native_graph_storage::NativeGraphStorage;
use crate::symbol_storage::SymbolStorage;
use std::path::Path;

/// Create a production-ready symbol storage with all wrappers
///
/// DEPRECATED: Use binary symbol format (BinarySymbolWriter/Reader) instead.
/// Binary format is 10x faster than JSON-based SymbolStorage.
///
/// Returns a symbol storage instance wrapped with:
/// - Tracing for observability
/// - Validation for input safety
/// - Retry logic for resilience
/// - Caching for performance
///
/// # Arguments
/// * `data_dir` - Directory for storing data
/// * `cache_size` - Optional cache size (defaults to 1000)
#[deprecated(
    note = "Use BinarySymbolWriter/Reader for 10x better performance. See ingest_with_binary_symbols_and_relationships."
)]
pub async fn create_symbol_storage(
    data_dir: &str,
    cache_size: Option<usize>,
) -> Result<Arc<Mutex<SymbolStorage>>> {
    // Create base storage with all wrappers
    let storage = create_file_storage(data_dir, cache_size).await?;

    // Create symbol storage with wrapped storage
    let symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

    Ok(Arc::new(Mutex::new(symbol_storage)))
}

/// Create a test symbol storage for unit tests
///
/// DEPRECATED: Use binary symbol format for tests.
///
/// Returns a symbol storage backed by temporary directory storage
#[deprecated(note = "Use BinarySymbolWriter/Reader for tests")]
pub async fn create_test_symbol_storage() -> Result<Arc<Mutex<SymbolStorage>>> {
    // Use temporary directory for test storage
    let test_dir = format!("test_data/symbol_test_{}", Uuid::new_v4());
    tokio::fs::create_dir_all(&test_dir).await?;

    let storage = create_file_storage(&test_dir, Some(100)).await?;
    let symbol_storage = SymbolStorage::new(Box::new(storage)).await?;

    // Clean up will happen when test ends
    Ok(Arc::new(Mutex::new(symbol_storage)))
}

/// Create a symbol storage with custom underlying storage
///
/// DEPRECATED: Use binary symbol format instead.
///
/// Allows providing a custom storage implementation while still
/// getting the full symbol extraction and indexing capabilities
#[deprecated(note = "Use BinarySymbolWriter/Reader instead")]
pub async fn create_symbol_storage_with_storage(
    storage: Box<dyn Storage + Send + Sync>,
) -> Result<Arc<Mutex<SymbolStorage>>> {
    let symbol_storage = SymbolStorage::new(storage).await?;
    Ok(Arc::new(Mutex::new(symbol_storage)))
}

/// Create a symbol storage with both document and graph storage backends
///
/// DEPRECATED: Use binary symbol format with dependency_graph.bin instead.
///
/// This enables dual storage architecture for optimal performance:
/// - Document storage for symbol metadata and content
/// - Graph storage for O(1) relationship lookups
///
/// # Arguments
/// * `data_dir` - Directory for storing data
/// * `cache_size` - Optional cache size (defaults to 1000)
#[deprecated(note = "Use binary symbols.kota and dependency_graph.bin files instead")]
pub async fn create_symbol_storage_with_graph(
    data_dir: &str,
    cache_size: Option<usize>,
) -> Result<Arc<Mutex<SymbolStorage>>> {
    // Create document storage with all wrappers
    let document_storage = create_file_storage(data_dir, cache_size).await?;

    // Create graph storage for relationships
    let graph_path = Path::new(data_dir).join("graph");
    tokio::fs::create_dir_all(&graph_path).await?;
    let graph_config = GraphStorageConfig::default();
    let graph_storage = NativeGraphStorage::new(graph_path, graph_config).await?;

    // Create symbol storage with both backends
    let symbol_storage =
        SymbolStorage::with_graph_storage(Box::new(document_storage), Box::new(graph_storage))
            .await?;

    Ok(Arc::new(Mutex::new(symbol_storage)))
}
