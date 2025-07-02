// Tests for Builder Patterns - Stage 6
// These tests ensure that our builders provide ergonomic APIs with proper validation

use kotadb::builders::*;
use kotadb::{Document, Query};
use anyhow::Result;

#[test]
fn test_document_builder_basic() -> Result<()> {
    let doc = DocumentBuilder::new()
        .path("/test/document.md")?
        .title("Test Document")?
        .content(b"Hello, world!")
        .word_count(2)
        .build()?;
    
    assert_eq!(doc.path, "/test/document.md");
    assert_eq!(doc.title, "Test Document");
    assert_eq!(doc.size, 13);
    assert_eq!(doc.word_count, 2);
    assert!(doc.created > 0);
    assert!(doc.updated >= doc.created);
    
    Ok(())
}

#[test]
fn test_document_builder_auto_word_count() -> Result<()> {
    let content = b"This is a test document with several words in it.";
    let doc = DocumentBuilder::new()
        .path("/test/auto_count.md")?
        .title("Auto Count")?
        .content(content)
        // Don't set word_count - should be calculated
        .build()?;
    
    assert_eq!(doc.word_count, 10); // Automatically counted
    assert_eq!(doc.size, content.len() as u64);
    
    Ok(())
}

#[test]
fn test_document_builder_custom_timestamps() -> Result<()> {
    let doc = DocumentBuilder::new()
        .path("/test/timed.md")?
        .title("Timed Document")?
        .content(b"Content")
        .timestamps(1000, 2000)?
        .build()?;
    
    assert_eq!(doc.created, 1000);
    assert_eq!(doc.updated, 2000);
    
    Ok(())
}

#[test]
fn test_document_builder_validation() {
    // Missing required fields
    let result = DocumentBuilder::new().build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("path is required"));
    
    let result = DocumentBuilder::new()
        .path("/test/doc.md").unwrap()
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title is required"));
    
    let result = DocumentBuilder::new()
        .path("/test/doc.md").unwrap()
        .title("Title").unwrap()
        .build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("content is required"));
    
    // Invalid timestamps
    let result = DocumentBuilder::new()
        .path("/test/doc.md").unwrap()
        .title("Title").unwrap()
        .content(b"Content")
        .timestamps(2000, 1000); // updated < created
    assert!(result.is_err());
}

#[test]
fn test_query_builder_text_only() -> Result<()> {
    let query = QueryBuilder::new()
        .with_text("search term")?
        .build()?;
    
    assert_eq!(query.text, Some("search term".to_string()));
    assert!(query.tags.is_none());
    assert!(query.date_range.is_none());
    assert_eq!(query.limit, 10); // Default
    
    Ok(())
}

#[test]
fn test_query_builder_with_tags() -> Result<()> {
    let query = QueryBuilder::new()
        .with_tag("rust")?
        .with_tag("database")?
        .build()?;
    
    assert!(query.text.is_none());
    assert_eq!(query.tags.as_ref().unwrap().len(), 2);
    assert!(query.tags.as_ref().unwrap().contains(&"rust".to_string()));
    assert!(query.tags.as_ref().unwrap().contains(&"database".to_string()));
    
    Ok(())
}

#[test]
fn test_query_builder_with_multiple_tags() -> Result<()> {
    let tags = vec!["rust", "database", "distributed"];
    let query = QueryBuilder::new()
        .with_tags(tags)?
        .build()?;
    
    assert_eq!(query.tags.as_ref().unwrap().len(), 3);
    
    Ok(())
}

#[test]
fn test_query_builder_full() -> Result<()> {
    let query = QueryBuilder::new()
        .with_text("search term")?
        .with_tag("rust")?
        .with_date_range(1000, 2000)?
        .with_limit(50)?
        .build()?;
    
    assert_eq!(query.text, Some("search term".to_string()));
    assert_eq!(query.tags.as_ref().unwrap().len(), 1);
    assert_eq!(query.date_range, Some((1000, 2000)));
    assert_eq!(query.limit, 50);
    
    Ok(())
}

#[test]
fn test_query_builder_validation() {
    // Empty text
    let result = QueryBuilder::new()
        .with_text("");
    assert!(result.is_err());
    
    // Invalid date range
    let result = QueryBuilder::new()
        .with_text("test").unwrap()
        .with_date_range(2000, 1000); // end < start
    assert!(result.is_err());
    
    // Invalid limit
    let result = QueryBuilder::new()
        .with_text("test").unwrap()
        .with_limit(0);
    assert!(result.is_err());
    
    let result = QueryBuilder::new()
        .with_text("test").unwrap()
        .with_limit(10000);
    assert!(result.is_err());
}

#[test]
fn test_storage_config_builder() -> Result<()> {
    use std::time::Duration;
    
    let config = StorageConfigBuilder::new()
        .path("/data/kotadb")?
        .cache_size(200 * 1024 * 1024)
        .sync_interval(Duration::from_secs(30))
        .compression(true)
        .build()?;
    
    assert_eq!(config.path.as_str(), "/data/kotadb");
    assert_eq!(config.cache_size, Some(200 * 1024 * 1024));
    assert_eq!(config.sync_interval, Some(Duration::from_secs(30)));
    assert!(config.compression_enabled);
    assert!(config.encryption_key.is_none());
    
    Ok(())
}

#[test]
fn test_storage_config_builder_no_cache() -> Result<()> {
    let config = StorageConfigBuilder::new()
        .path("/data/kotadb")?
        .no_cache()
        .build()?;
    
    assert!(config.cache_size.is_none());
    
    Ok(())
}

#[test]
fn test_storage_config_builder_with_encryption() -> Result<()> {
    let key = [0u8; 32];
    let config = StorageConfigBuilder::new()
        .path("/data/secure")?
        .encryption_key(key)
        .build()?;
    
    assert_eq!(config.encryption_key, Some(key));
    
    Ok(())
}

#[test]
fn test_index_config_builder() -> Result<()> {
    let config = IndexConfigBuilder::new()
        .name("trigram_index")
        .max_memory(100 * 1024 * 1024)
        .persistence(true)
        .fuzzy_search(true)
        .similarity_threshold(0.85)?
        .build()?;
    
    assert_eq!(config.name, "trigram_index");
    assert_eq!(config.max_memory, Some(100 * 1024 * 1024));
    assert!(config.persistence_enabled);
    assert!(config.fuzzy_search);
    assert_eq!(config.similarity_threshold, 0.85);
    
    Ok(())
}

#[test]
fn test_index_config_builder_validation() {
    // Missing name
    let result = IndexConfigBuilder::new().build();
    assert!(result.is_err());
    
    // Invalid similarity threshold
    let result = IndexConfigBuilder::new()
        .name("test")
        .similarity_threshold(-0.1);
    assert!(result.is_err());
    
    let result = IndexConfigBuilder::new()
        .name("test")
        .similarity_threshold(1.5);
    assert!(result.is_err());
}

#[test]
fn test_metrics_builder() -> Result<()> {
    let metrics = MetricsBuilder::new()
        .document_count(1000)
        .total_size(10 * 1024 * 1024)
        .index_size("trigram", 1 * 1024 * 1024)
        .index_size("tag", 500 * 1024)
        .build()?;
    
    assert_eq!(metrics.document_count, 1000);
    assert_eq!(metrics.total_size_bytes, 10 * 1024 * 1024);
    assert_eq!(metrics.index_sizes.len(), 2);
    assert_eq!(metrics.index_sizes["trigram"], 1 * 1024 * 1024);
    assert_eq!(metrics.index_sizes["tag"], 500 * 1024);
    
    Ok(())
}

#[test]
fn test_metrics_builder_validation() {
    // Invalid metrics (size < count)
    let result = MetricsBuilder::new()
        .document_count(1000)
        .total_size(100) // Too small for 1000 docs
        .build();
    assert!(result.is_err());
}

#[test]
fn test_builder_defaults() -> Result<()> {
    // StorageConfigBuilder has sensible defaults
    let config = StorageConfigBuilder::default()
        .path("/data/kotadb")?
        .build()?;
    
    assert_eq!(config.cache_size, Some(100 * 1024 * 1024)); // 100MB default
    assert!(config.compression_enabled); // Compression on by default
    
    // IndexConfigBuilder has sensible defaults
    let config = IndexConfigBuilder::default()
        .name("test_index")
        .build()?;
    
    assert_eq!(config.max_memory, Some(50 * 1024 * 1024)); // 50MB default
    assert!(config.persistence_enabled); // Persistence on by default
    assert!(config.fuzzy_search); // Fuzzy search on by default
    assert_eq!(config.similarity_threshold, 0.8); // 80% similarity default
    
    Ok(())
}

#[test]
fn test_builder_chaining() -> Result<()> {
    // All methods should chain properly
    let doc = DocumentBuilder::new()
        .path("/test/chain.md")?
        .title("Chained Document")?
        .content(b"Chained content")
        .word_count(2)
        .timestamps(1000, 2000)?
        .build()?;
    
    assert_eq!(doc.path, "/test/chain.md");
    assert_eq!(doc.title, "Chained Document");
    assert_eq!(doc.word_count, 2);
    assert_eq!(doc.created, 1000);
    assert_eq!(doc.updated, 2000);
    
    Ok(())
}

#[test]
fn test_builder_error_propagation() {
    // Errors in builder methods should propagate properly
    let result = DocumentBuilder::new()
        .path("") // Invalid empty path
        .map(|_| ());
    assert!(result.is_err());
    
    let result = DocumentBuilder::new()
        .path("/valid/path.md").unwrap()
        .title("") // Invalid empty title
        .map(|_| ());
    assert!(result.is_err());
    
    let result = DocumentBuilder::new()
        .path("/valid/path.md").unwrap()
        .title("Valid Title").unwrap()
        .timestamps(-1, 1000) // Invalid negative timestamp
        .map(|_| ());
    assert!(result.is_err());
}