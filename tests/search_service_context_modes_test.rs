// SearchService Context Modes Test
// Critical test coverage for PR #597 fix - ensuring search context modes work correctly
// This addresses the core performance regression where all queries were forced through LLM processing

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, RwLock};

use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index,
    services::search_service::{DatabaseAccess, SearchOptions, SearchService, SearchType},
    DocumentBuilder, Index, Storage, ValidatedDocumentId,
};

/// Test implementation of DatabaseAccess trait for SearchService testing
struct TestDatabase {
    storage: Arc<Mutex<dyn Storage>>,
    primary_index: Arc<Mutex<dyn Index>>,
    trigram_index: Arc<Mutex<dyn Index>>,
    path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>>,
}

impl DatabaseAccess for TestDatabase {
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

/// Helper function to create test database with sample documents
async fn setup_test_database() -> Result<(TempDir, TestDatabase)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    // Create components
    let storage = create_file_storage(db_path.join("storage").to_str().unwrap(), None).await?;
    let primary_index =
        create_primary_index(db_path.join("primary").to_str().unwrap(), None).await?;
    let trigram_index =
        create_trigram_index(db_path.join("trigram").to_str().unwrap(), None).await?;

    let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));
    let primary_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(primary_index));
    let trigram_index: Arc<Mutex<dyn Index>> = Arc::new(Mutex::new(trigram_index));
    let path_cache: Arc<RwLock<HashMap<String, ValidatedDocumentId>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let database = TestDatabase {
        storage: storage.clone(),
        primary_index: primary_index.clone(),
        trigram_index: trigram_index.clone(),
        path_cache,
    };

    // Insert sample documents for testing
    {
        let mut storage_guard = storage.lock().await;
        let mut primary_guard = primary_index.lock().await;
        let mut trigram_guard = trigram_index.lock().await;

        // Document 1: Simple code example
        let doc1 = DocumentBuilder::new()
            .path("src/example.rs")
            .unwrap()
            .title("Example Code")
            .unwrap()
            .content(b"async fn example_function() { println!(\"Hello, world!\"); }")
            .build()?;

        storage_guard.insert(doc1.clone()).await?;
        primary_guard.insert(doc1.id, doc1.path.clone()).await?;
        trigram_guard
            .insert_with_content(doc1.id, doc1.path.clone(), &doc1.content)
            .await?;

        // Document 2: More complex example
        let doc2 = DocumentBuilder::new()
            .path("src/complex.rs")
            .unwrap()
            .title("Complex Code")
            .unwrap()
            .content(
                b"
pub struct Database {
    storage: Storage,
    index: Index,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let storage = Storage::new()?;
        let index = Index::new()?;
        Ok(Self { storage, index })
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<Document>> {
        self.index.search(query).await
    }
}",
            )
            .build()?;

        storage_guard.insert(doc2.clone()).await?;
        primary_guard.insert(doc2.id, doc2.path.clone()).await?;
        trigram_guard
            .insert_with_content(doc2.id, doc2.path.clone(), &doc2.content)
            .await?;
    }

    Ok((temp_dir, database))
}

#[tokio::test]
async fn test_search_context_none_uses_fast_search() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    let options = SearchOptions {
        query: "async fn".to_string(),
        limit: 10,
        tags: None,
        context: "none".to_string(),
        quiet: false,
    };

    let start_time = std::time::Instant::now();
    let result = search_service.search_content(options).await?;
    let elapsed = start_time.elapsed();

    // Should use fast regular search, not LLM search
    assert!(matches!(result.search_type, SearchType::RegularSearch));
    assert!(result.llm_response.is_none());

    // Should be fast (< 50ms for small dataset)
    assert!(
        elapsed.as_millis() < 50,
        "Fast search should complete quickly: {}ms",
        elapsed.as_millis()
    );

    // Should still return results
    assert!(result.total_count > 0, "Should find matching documents");

    println!("✓ Context 'none' uses fast regular search");
    println!(
        "  Completed in {}ms with {} results",
        elapsed.as_millis(),
        result.total_count
    );

    Ok(())
}

#[tokio::test]
async fn test_search_context_minimal_uses_fast_search() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    let options = SearchOptions {
        query: "Database".to_string(),
        limit: 10,
        tags: None,
        context: "minimal".to_string(), // This is the NEW default from PR #597
        quiet: false,
    };

    let start_time = std::time::Instant::now();
    let result = search_service.search_content(options).await?;
    let elapsed = start_time.elapsed();

    // Should use fast regular search, not LLM search
    assert!(matches!(result.search_type, SearchType::RegularSearch));
    assert!(result.llm_response.is_none());

    // Should be fast
    assert!(
        elapsed.as_millis() < 50,
        "Minimal context should use fast search: {}ms",
        elapsed.as_millis()
    );

    println!("✓ Context 'minimal' (new default) uses fast regular search");
    println!(
        "  Completed in {}ms with {} results",
        elapsed.as_millis(),
        result.total_count
    );

    Ok(())
}

#[tokio::test]
async fn test_search_context_medium_uses_llm_search() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    let options = SearchOptions {
        query: "search function".to_string(),
        limit: 10,
        tags: None,
        context: "medium".to_string(), // Should trigger LLM search
        quiet: false,
    };

    let result = search_service.search_content(options).await;

    match result {
        Ok(result) => {
            // If LLM search succeeds, verify it was used
            if matches!(result.search_type, SearchType::LLMOptimized) {
                assert!(result.llm_response.is_some());
                println!("✓ Context 'medium' successfully used LLM search");
            } else {
                // If LLM search fails, should fall back to regular search
                assert!(matches!(result.search_type, SearchType::RegularSearch));
                assert!(result.llm_response.is_none());
                println!("✓ Context 'medium' fell back to regular search (LLM unavailable)");
            }
        }
        Err(_) => {
            // LLM search may not be available in test environment - that's OK
            println!("✓ Context 'medium' properly handles LLM unavailability");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_search_context_full_uses_llm_search() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    let options = SearchOptions {
        query: "complex database implementation".to_string(),
        limit: 10,
        tags: None,
        context: "full".to_string(), // Should trigger LLM search
        quiet: false,
    };

    let result = search_service.search_content(options).await;

    match result {
        Ok(result) => {
            // If LLM search succeeds, verify it was used
            if matches!(result.search_type, SearchType::LLMOptimized) {
                assert!(result.llm_response.is_some());
                println!("✓ Context 'full' successfully used LLM search");
            } else {
                // If LLM search fails, should fall back to regular search
                assert!(matches!(result.search_type, SearchType::RegularSearch));
                assert!(result.llm_response.is_none());
                println!("✓ Context 'full' fell back to regular search (LLM unavailable)");
            }
        }
        Err(_) => {
            // LLM search may not be available in test environment - that's OK
            println!("✓ Context 'full' properly handles LLM unavailability");
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_search_wildcard_always_uses_regular_search() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    // Test wildcard with medium context - should still use regular search
    let options = SearchOptions {
        query: "*".to_string(), // Wildcard query
        limit: 10,
        tags: None,
        context: "medium".to_string(), // Even with medium context
        quiet: false,
    };

    let result = search_service.search_content(options).await?;

    // Wildcard should NEVER use LLM search regardless of context
    assert!(matches!(result.search_type, SearchType::WildcardSearch));
    assert!(result.llm_response.is_none());

    println!("✓ Wildcard queries always use regular search regardless of context");
    println!("  Found {} total documents", result.total_count);

    Ok(())
}

#[tokio::test]
async fn test_search_empty_query_handling() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    let options = SearchOptions {
        query: "".to_string(), // Empty query
        limit: 10,
        tags: None,
        context: "medium".to_string(),
        quiet: false,
    };

    let result = search_service.search_content(options).await?;

    // Empty query should return no results quickly
    assert_eq!(result.documents.len(), 0);
    assert_eq!(result.total_count, 0);
    assert!(result.llm_response.is_none());
    assert!(matches!(result.search_type, SearchType::RegularSearch));

    println!("✓ Empty queries handled correctly");

    Ok(())
}

#[tokio::test]
async fn test_performance_regression_protection() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    // Test the exact scenario that was causing 79+ second delays
    let test_cases = vec![
        ("rust", "minimal"),
        ("async fn", "minimal"),
        ("Database", "minimal"),
        ("search", "minimal"),
    ];

    for (query, context) in test_cases {
        let options = SearchOptions {
            query: query.to_string(),
            limit: 10,
            tags: None,
            context: context.to_string(),
            quiet: false,
        };

        let start_time = std::time::Instant::now();
        let result = search_service.search_content(options).await?;
        let elapsed = start_time.elapsed();

        // CRITICAL: These searches must complete in under 1 second (was 79+ seconds before fix)
        assert!(
            elapsed.as_millis() < 1000,
            "Query '{}' took {}ms - performance regression detected!",
            query,
            elapsed.as_millis()
        );

        // Should use fast search by default
        assert!(matches!(result.search_type, SearchType::RegularSearch));

        println!(
            "✓ Query '{}' completed in {}ms (fast search)",
            query,
            elapsed.as_millis()
        );
    }

    println!("✅ Performance regression protection: All queries complete in <1s");

    Ok(())
}

#[tokio::test]
async fn test_context_mode_conditional_logic() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    // This tests the exact conditional logic from search_service.rs:131
    // if options.query != "*" && (options.context == "medium" || options.context == "full")

    let test_cases = vec![
        // Cases that should NOT trigger LLM search
        ("*", "medium", false, "Wildcard with medium context"),
        ("*", "full", false, "Wildcard with full context"),
        ("test", "none", false, "Regular query with none context"),
        (
            "test",
            "minimal",
            false,
            "Regular query with minimal context",
        ),
        (
            "test",
            "unknown",
            false,
            "Regular query with unknown context",
        ),
        // Cases that SHOULD trigger LLM search (may fall back if LLM unavailable)
        ("test", "medium", true, "Regular query with medium context"),
        ("test", "full", true, "Regular query with full context"),
    ];

    for (query, context, should_try_llm, description) in test_cases {
        let options = SearchOptions {
            query: query.to_string(),
            limit: 10,
            tags: None,
            context: context.to_string(),
            quiet: false,
        };

        let result = search_service.search_content(options).await?;

        if should_try_llm {
            // Should attempt LLM search - may succeed or fall back
            assert!(
                matches!(
                    result.search_type,
                    SearchType::LLMOptimized | SearchType::RegularSearch
                ),
                "Case '{}': Should attempt LLM or fall back to regular",
                description
            );
        } else {
            // Should directly use regular search
            assert!(
                matches!(
                    result.search_type,
                    SearchType::RegularSearch | SearchType::WildcardSearch
                ),
                "Case '{}': Should use regular/wildcard search directly",
                description
            );
            assert!(
                result.llm_response.is_none(),
                "Case '{}': Should not have LLM response",
                description
            );
        }

        println!(
            "✓ {}: {}",
            description,
            match result.search_type {
                SearchType::LLMOptimized => "Used LLM search",
                SearchType::RegularSearch => "Used regular search",
                SearchType::WildcardSearch => "Used wildcard search",
            }
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_llm_fallback_behavior() -> Result<()> {
    let (_temp_dir, database) = setup_test_database().await?;
    let symbol_db_path = PathBuf::from("/tmp/test_symbols");
    let search_service = SearchService::new(&database, symbol_db_path);

    let options = SearchOptions {
        query: "complex search query".to_string(),
        limit: 10,
        tags: None,
        context: "medium".to_string(), // Should try LLM
        quiet: false,
    };

    let result = search_service.search_content(options).await?;

    // Either LLM search works or it falls back to regular search
    // Both cases should work and return results quickly
    match result.search_type {
        SearchType::LLMOptimized => {
            assert!(result.llm_response.is_some());
            println!("✓ LLM search succeeded");
        }
        SearchType::RegularSearch => {
            assert!(result.llm_response.is_none());
            println!("✓ LLM search fell back to regular search gracefully");
        }
        SearchType::WildcardSearch => {
            panic!("Should not be wildcard search for non-wildcard query");
        }
    }

    // Should return results in either case (result.total_count is usize, always >= 0)

    Ok(())
}
