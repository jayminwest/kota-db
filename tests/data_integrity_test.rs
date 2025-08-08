// Data Integrity Integration Tests - Stage 1: TDD for Phase 3 Production Readiness
// Tests ACID properties, data consistency, corruption detection, and data validation

use anyhow::Result;
use kotadb::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::task;
use uuid::Uuid;

/// Test ACID Atomicity - all operations in a transaction succeed or all fail
#[tokio::test]
async fn test_acid_atomicity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("atomicity_storage");
    let index_path = temp_dir.path().join("atomicity_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing ACID Atomicity properties...");

    // Create test documents for atomic operations
    let docs = create_test_documents(10, "atomicity")?;

    // Phase 1: Successful atomic transaction
    println!("  - Testing successful atomic transaction...");
    let transaction_docs = &docs[..5];

    // Simulate atomic operation - either all succeed or none do
    let mut transaction_successful = true;
    let mut committed_ids = Vec::new();

    // Begin transaction simulation
    let transaction_start = Instant::now();

    for (i, doc) in transaction_docs.iter().enumerate() {
        // Simulate potential failure on the 3rd operation (but don't actually fail)
        match storage.insert(doc.clone()).await {
            Ok(()) => match optimized_index.insert(doc.id, doc.path.clone()).await {
                Ok(()) => {
                    committed_ids.push(doc.id);
                    println!("    - Operation {}/5 succeeded: {}", i + 1, doc.id);
                }
                Err(e) => {
                    println!("    - Index operation failed at {}: {}", i + 1, e);
                    transaction_successful = false;
                    break;
                }
            },
            Err(e) => {
                println!("    - Storage operation failed at {}: {}", i + 1, e);
                transaction_successful = false;
                break;
            }
        }
    }

    let transaction_duration = transaction_start.elapsed();
    println!(
        "  - Transaction completed: success={}, operations={}, duration={:?}",
        transaction_successful,
        committed_ids.len(),
        transaction_duration
    );

    if transaction_successful {
        // Verify all documents are present and accessible
        for doc_id in &committed_ids {
            let retrieved = storage.get(doc_id).await?;
            assert!(
                retrieved.is_some(),
                "Document missing after successful transaction: {doc_id}"
            );
        }

        // Verify consistency between storage and index
        let storage_count = storage.list_all().await?.len();
        assert_eq!(
            storage_count,
            committed_ids.len(),
            "Storage and committed operations count mismatch"
        );
    }

    // Phase 2: Simulated failed atomic transaction with rollback
    println!("  - Testing failed transaction rollback simulation...");

    let rollback_docs = &docs[5..8];
    let mut partial_commits = Vec::new();

    // Simulate transaction that fails partway through
    for (i, doc) in rollback_docs.iter().enumerate() {
        if i == 2 {
            // Simulate failure on 3rd operation
            println!("    - Simulated failure at operation 3/3");
            break;
        }

        // These operations would need to be rolled back in a real implementation
        storage.insert(doc.clone()).await?;
        optimized_index.insert(doc.id, doc.path.clone()).await?;
        partial_commits.push(doc.id);
        println!(
            "    - Partial operation {} succeeded (needs rollback): {}",
            i + 1,
            doc.id
        );
    }

    // Simulate rollback by removing partially committed operations
    println!(
        "    - Rolling back {} partial operations...",
        partial_commits.len()
    );
    for doc_id in &partial_commits {
        storage.delete(doc_id).await?;
        optimized_index.delete(doc_id).await?;
    }

    // Verify rollback completed - no partial commits should remain
    for doc_id in &partial_commits {
        let retrieved = storage.get(doc_id).await?;
        assert!(
            retrieved.is_none(),
            "Partially committed document not rolled back: {doc_id}"
        );
    }

    // Final state should only contain successful transaction documents
    let final_docs = storage.list_all().await?;
    assert_eq!(
        final_docs.len(),
        committed_ids.len(),
        "Final document count incorrect after rollback"
    );

    Ok(())
}

/// Test ACID Consistency - data integrity constraints are maintained
#[tokio::test]
async fn test_acid_consistency() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("consistency_storage");
    let index_path = temp_dir.path().join("consistency_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing ACID Consistency properties...");

    // Phase 1: Test referential consistency
    let docs = create_test_documents(20, "consistency")?;

    // Insert documents and build reference map
    let mut reference_map: HashMap<ValidatedDocumentId, ValidatedPath> = HashMap::new();

    for doc in &docs {
        storage.insert(doc.clone()).await?;
        optimized_index.insert(doc.id, doc.path.clone()).await?;
        reference_map.insert(doc.id, doc.path.clone());
    }

    println!("  - Inserted {} documents", docs.len());

    // Phase 2: Verify storage-index consistency
    println!("  - Verifying storage-index consistency...");
    let storage_docs = storage.list_all().await?;

    for storage_doc in &storage_docs {
        // Every document in storage should be findable via reference
        assert!(
            reference_map.contains_key(&storage_doc.id),
            "Storage document not in reference map: {}",
            storage_doc.id
        );

        // Path consistency check
        let expected_path = &reference_map[&storage_doc.id];
        assert_eq!(
            &storage_doc.path, expected_path,
            "Path mismatch for document {}: expected {:?}, got {:?}",
            storage_doc.id, expected_path, storage_doc.path
        );
    }

    // Phase 3: Test constraint validation during updates
    println!("  - Testing constraint validation during updates...");

    let update_doc = &docs[0];
    let mut updated_doc = update_doc.clone();

    // Valid update - should maintain consistency
    updated_doc.updated_at = chrono::Utc::now();
    let original_content = updated_doc.content.clone();
    updated_doc.content = b"Updated content for consistency test".to_vec();
    updated_doc.size = updated_doc.content.len();

    storage.update(updated_doc.clone()).await?; // Use update for existing documents

    // Verify update maintained consistency
    let retrieved_update = storage.get(&updated_doc.id).await?;
    assert!(retrieved_update.is_some(), "Updated document not found");

    let retrieved = retrieved_update.unwrap();
    assert_eq!(
        retrieved.content, updated_doc.content,
        "Content not updated properly"
    );
    assert_ne!(
        retrieved.content, original_content,
        "Content didn't actually change"
    );
    assert_eq!(
        retrieved.size,
        updated_doc.content.len(),
        "Size not updated consistently"
    );

    // Phase 4: Test deletion consistency
    println!("  - Testing deletion consistency...");

    let delete_doc = &docs[1];
    let delete_id = delete_doc.id;

    // Delete from both storage and index
    let storage_deleted = storage.delete(&delete_id).await?;
    let index_deleted = optimized_index.delete(&delete_id).await?;

    assert!(storage_deleted, "Storage deletion failed");
    assert!(index_deleted, "Index deletion failed");

    // Verify consistent deletion
    let retrieved_deleted = storage.get(&delete_id).await?;
    assert!(
        retrieved_deleted.is_none(),
        "Deleted document still in storage"
    );

    // Verify reference map consistency after deletion
    let remaining_storage_docs = storage.list_all().await?;
    let remaining_count = remaining_storage_docs.len();
    let expected_remaining = docs.len() - 1; // One deleted

    assert_eq!(
        remaining_count, expected_remaining,
        "Remaining document count inconsistent after deletion"
    );

    // Phase 5: Test concurrent consistency (simplified)
    println!("  - Testing concurrent operation consistency...");

    let concurrent_docs = create_test_documents(10, "concurrent")?;
    let shared_storage = Arc::new(tokio::sync::Mutex::new(storage));
    let shared_index = Arc::new(tokio::sync::Mutex::new(optimized_index));

    let mut handles = Vec::new();

    // Spawn concurrent operations
    for (i, doc) in concurrent_docs.iter().enumerate() {
        let storage_ref = Arc::clone(&shared_storage);
        let index_ref = Arc::clone(&shared_index);
        let doc_clone = doc.clone();

        let handle = task::spawn(async move {
            // Each task performs insert and immediate read
            {
                let mut storage_guard = storage_ref.lock().await;
                storage_guard.insert(doc_clone.clone()).await?;
            }
            {
                let mut index_guard = index_ref.lock().await;
                index_guard
                    .insert(doc_clone.id, doc_clone.path.clone())
                    .await?;
            }

            // Verify immediate consistency
            {
                let storage_guard = storage_ref.lock().await;
                let retrieved = storage_guard.get(&doc_clone.id).await?;
                if retrieved.is_none() {
                    anyhow::bail!(
                        "Document not immediately available after insert: {}",
                        doc_clone.id
                    );
                }
            }

            Ok::<usize, anyhow::Error>(i)
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations
    let mut successful_ops = 0;
    for handle in handles {
        match handle.await? {
            Ok(_) => successful_ops += 1,
            Err(e) => println!("    - Concurrent operation failed: {e}"),
        }
    }

    println!(
        "  - Concurrent operations: {}/{} successful",
        successful_ops,
        concurrent_docs.len()
    );

    // Verify final consistency after concurrent operations
    let final_storage = shared_storage.lock().await;
    let final_docs = final_storage.list_all().await?;

    // Should have at least the successful concurrent operations plus previous docs
    assert!(
        final_docs.len() >= successful_ops,
        "Final document count too low after concurrent operations"
    );

    Ok(())
}

/// Test ACID Isolation - concurrent transactions don't interfere
#[tokio::test]
async fn test_acid_isolation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("isolation_storage");
    let index_path = temp_dir.path().join("isolation_index");

    let storage = Arc::new(tokio::sync::Mutex::new(
        create_file_storage(&storage_path.to_string_lossy(), Some(2000)).await?,
    ));
    let index = Arc::new(tokio::sync::Mutex::new({
        let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(2000)).await?;
        create_optimized_index_with_defaults(primary_index)
    }));

    println!("Testing ACID Isolation properties...");

    // Phase 1: Test read isolation - reads don't see uncommitted writes
    println!("  - Testing read isolation...");

    let isolation_docs = create_test_documents(50, "isolation")?;
    let batch_size = 10;

    // Split documents into batches for different "transactions"
    let batches: Vec<_> = isolation_docs.chunks(batch_size).collect();
    let mut handles = Vec::new();

    for (batch_id, batch) in batches.iter().enumerate() {
        let storage_ref = Arc::clone(&storage);
        let index_ref = Arc::clone(&index);
        let batch_docs = batch.to_vec();

        let handle = task::spawn(async move {
            let mut batch_results = Vec::new();

            // Each batch operates independently (simulating isolation)
            for doc in &batch_docs {
                let insert_start = Instant::now();

                // Insert with isolation (atomic lock acquisition)
                {
                    let mut storage_guard = storage_ref.lock().await;
                    let mut index_guard = index_ref.lock().await;

                    // Simulate transaction isolation by holding both locks
                    match storage_guard.insert(doc.clone()).await {
                        Ok(()) => {
                            match index_guard.insert(doc.id, doc.path.clone()).await {
                                Ok(()) => {
                                    batch_results.push((doc.id, true, insert_start.elapsed()));
                                }
                                Err(e) => {
                                    println!("      - Batch {batch_id} index failure: {e}");
                                    // In a real implementation, would rollback storage insert
                                    storage_guard.delete(&doc.id).await.ok();
                                    batch_results.push((doc.id, false, insert_start.elapsed()));
                                }
                            }
                        }
                        Err(e) => {
                            println!("      - Batch {batch_id} storage failure: {e}");
                            batch_results.push((doc.id, false, insert_start.elapsed()));
                        }
                    }
                }

                // Small delay to increase chance of interleaving
                tokio::time::sleep(Duration::from_millis(1)).await;
            }

            Ok::<(usize, Vec<(ValidatedDocumentId, bool, Duration)>), anyhow::Error>((
                batch_id,
                batch_results,
            ))
        });

        handles.push(handle);
    }

    // Collect results from all isolated batches
    let mut all_successful_ids = HashSet::new();
    let mut total_operations = 0;
    let mut successful_operations = 0;

    for handle in handles {
        let (batch_id, batch_results) = handle.await??;

        for (doc_id, success, _duration) in batch_results {
            total_operations += 1;
            if success {
                successful_operations += 1;
                all_successful_ids.insert(doc_id);
            }
        }

        println!("    - Batch {batch_id} completed");
    }

    println!(
        "  - Isolation test: {successful_operations}/{total_operations} operations successful"
    );

    // Verify isolation was maintained - no partial states visible
    let final_storage = storage.lock().await;
    let final_docs = final_storage.list_all().await?;

    // All documents in final state should be in successful set
    for doc in &final_docs {
        assert!(
            all_successful_ids.contains(&doc.id),
            "Document in final state not in successful operations: {}",
            doc.id
        );
    }

    assert_eq!(
        final_docs.len(),
        all_successful_ids.len(),
        "Final document count doesn't match successful operations"
    );

    Ok(())
}

/// Test ACID Durability - committed data survives system restart simulation
#[tokio::test]
async fn test_acid_durability() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("durability_storage");
    let index_path = temp_dir.path().join("durability_index");

    println!("Testing ACID Durability properties...");

    // Phase 1: Insert data and commit
    let docs = create_test_documents(25, "durability")?;
    let mut committed_ids = HashSet::new();

    {
        let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
        let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
        let mut optimized_index = create_optimized_index_with_defaults(primary_index);

        println!("  - Inserting and committing {} documents...", docs.len());

        for doc in &docs {
            storage.insert(doc.clone()).await?;
            optimized_index.insert(doc.id, doc.path.clone()).await?;
            committed_ids.insert(doc.id);
        }

        // Explicit sync to ensure durability
        storage.sync().await?;
        println!("  - Data committed and synced to storage");

        // Verify data is present before "crash"
        let pre_crash_docs = storage.list_all().await?;
        assert_eq!(
            pre_crash_docs.len(),
            docs.len(),
            "Not all documents committed"
        );

        // Storage and index go out of scope here (simulating shutdown)
    }

    // Phase 2: Simulate system restart by reopening storage
    println!("  - Simulating system restart...");

    {
        let storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
        let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
        let optimized_index = create_optimized_index_with_defaults(primary_index);

        println!("  - Storage reopened after restart");

        // Phase 3: Verify all committed data survived
        let post_restart_docs = storage.list_all().await?;

        println!(
            "  - Found {} documents after restart",
            post_restart_docs.len()
        );

        // Should have exactly the same documents
        assert_eq!(
            post_restart_docs.len(),
            committed_ids.len(),
            "Document count changed after restart: expected {}, got {}",
            committed_ids.len(),
            post_restart_docs.len()
        );

        // Every committed document should be recoverable
        for doc_id in &committed_ids {
            let retrieved = storage.get(doc_id).await?;
            assert!(
                retrieved.is_some(),
                "Committed document not durable after restart: {doc_id}"
            );

            // Verify content integrity
            let recovered_doc = retrieved.unwrap();
            let original_doc = docs.iter().find(|d| d.id == *doc_id).unwrap();

            assert_eq!(
                recovered_doc.content, original_doc.content,
                "Document content corrupted after restart: {doc_id}"
            );
            assert_eq!(
                recovered_doc.size, original_doc.size,
                "Document size corrupted after restart: {doc_id}"
            );
            assert_eq!(
                recovered_doc.path, original_doc.path,
                "Document path corrupted after restart: {doc_id}"
            );
        }

        // Phase 4: Test that system is fully functional after restart
        println!("  - Testing system functionality after restart...");

        let mut storage_mut = storage;
        let mut index_mut = optimized_index;

        // Insert new document after restart
        let post_restart_doc = create_test_documents(1, "post-restart")?;
        let new_doc = &post_restart_doc[0];

        storage_mut.insert(new_doc.clone()).await?;
        index_mut.insert(new_doc.id, new_doc.path.clone()).await?;

        // Verify new document is accessible
        let new_doc_retrieved = storage_mut.get(&new_doc.id).await?;
        assert!(
            new_doc_retrieved.is_some(),
            "New document not accessible after restart"
        );

        // Final document count should be original + 1
        let final_docs = storage_mut.list_all().await?;
        assert_eq!(
            final_docs.len(),
            committed_ids.len() + 1,
            "Final document count incorrect after restart operations"
        );
    }

    println!("  - Durability test completed successfully");

    Ok(())
}

/// Test data corruption detection and handling
#[tokio::test]
async fn test_data_corruption_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("corruption_storage");
    let index_path = temp_dir.path().join("corruption_index");

    let mut storage = create_file_storage(&storage_path.to_string_lossy(), Some(1000)).await?;
    let primary_index = create_primary_index(&index_path.to_string_lossy(), Some(1000)).await?;
    let mut optimized_index = create_optimized_index_with_defaults(primary_index);

    println!("Testing data corruption detection...");

    // Phase 1: Insert clean data
    let docs = create_test_documents(15, "corruption")?;

    for doc in &docs {
        storage.insert(doc.clone()).await?;
        optimized_index.insert(doc.id, doc.path.clone()).await?;
    }

    println!("  - Inserted {} clean documents", docs.len());

    // Phase 2: Test validation of document integrity
    for doc in &docs {
        let retrieved = storage.get(&doc.id).await?;
        assert!(retrieved.is_some(), "Document missing: {}", doc.id);

        let retrieved_doc = retrieved.unwrap();

        // Verify content integrity
        assert_eq!(
            retrieved_doc.content, doc.content,
            "Content corruption detected in {}",
            doc.id
        );

        // Verify size consistency
        assert_eq!(
            retrieved_doc.size,
            doc.content.len(),
            "Size inconsistency detected in {}",
            doc.id
        );
        assert_eq!(
            retrieved_doc.size, doc.size,
            "Size mismatch with original in {}",
            doc.id
        );

        // Verify metadata integrity
        assert_eq!(
            retrieved_doc.path, doc.path,
            "Path corruption detected in {}",
            doc.id
        );
        assert_eq!(
            retrieved_doc.title, doc.title,
            "Title corruption detected in {}",
            doc.id
        );
    }

    // Phase 3: Test detection of inconsistent states
    println!("  - Testing inconsistent state detection...");

    // Create a document that exists in storage but not in index (orphaned)
    let orphan_doc = create_test_documents(1, "orphan")?;
    let orphan = &orphan_doc[0];

    // Insert only to storage (not index) to create inconsistency
    storage.insert(orphan.clone()).await?;

    // Verify storage has the orphaned document
    let orphan_retrieved = storage.get(&orphan.id).await?;
    assert!(orphan_retrieved.is_some(), "Orphan document not in storage");

    // In a real system, we'd have a consistency checker that detects this
    let storage_docs = storage.list_all().await?;
    let expected_total = docs.len() + 1; // Original docs + orphan
    assert_eq!(
        storage_docs.len(),
        expected_total,
        "Storage document count unexpected"
    );

    // Phase 4: Test handling of malformed data
    println!("  - Testing malformed data handling...");

    // Attempt to create document with invalid data
    let invalid_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;

    // Create document with mismatched size field (content size != size field)
    let mut malformed_doc = docs[0].clone();
    malformed_doc.id = invalid_id;
    malformed_doc.content = b"Short content".to_vec();
    malformed_doc.size = 999999; // Deliberately wrong size

    // Insert malformed document
    storage.insert(malformed_doc.clone()).await?;

    // Retrieve and detect corruption
    let malformed_retrieved = storage.get(&invalid_id).await?;
    assert!(
        malformed_retrieved.is_some(),
        "Malformed document not stored"
    );

    let malformed = malformed_retrieved.unwrap();

    // Detect size corruption
    let actual_content_size = malformed.content.len();
    let declared_size = malformed.size;

    if actual_content_size != declared_size {
        println!(
            "    - Size corruption detected: declared={declared_size}, actual={actual_content_size}"
        );

        // In a real system, this would trigger corruption handling
        // For now, we'll correct it by updating the size
        let mut corrected_doc = malformed.clone();
        corrected_doc.size = actual_content_size;
        corrected_doc.updated_at = chrono::Utc::now(); // Update timestamp for validation

        storage.update(corrected_doc.clone()).await?;

        // Verify correction
        let corrected_retrieved = storage.get(&invalid_id).await?;
        let corrected = corrected_retrieved.unwrap();
        assert_eq!(
            corrected.size,
            corrected.content.len(),
            "Size correction failed"
        );
    }

    // Phase 5: Test checksum validation (simplified)
    println!("  - Testing content validation...");

    for doc in &docs[..5] {
        // Test subset for performance
        let retrieved = storage.get(&doc.id).await?;
        let retrieved_doc = retrieved.unwrap();

        // Simple content validation - verify it contains expected patterns
        let content_str = String::from_utf8_lossy(&retrieved_doc.content);

        // Should contain the document type
        assert!(
            content_str.contains("corruption"),
            "Document content doesn't contain expected pattern: {}",
            doc.id
        );

        // Should be valid UTF-8 (no binary corruption)
        let is_valid_utf8 = String::from_utf8(retrieved_doc.content.clone()).is_ok();
        assert!(
            is_valid_utf8,
            "Document content not valid UTF-8: {}",
            doc.id
        );

        // Content should not be empty
        assert!(
            !retrieved_doc.content.is_empty(),
            "Document content is empty: {}",
            doc.id
        );
    }

    println!("  - Data corruption detection tests completed");

    Ok(())
}

// Helper function to create test documents for integrity testing
fn create_test_documents(count: usize, test_type: &str) -> Result<Vec<Document>> {
    let mut documents = Vec::with_capacity(count);

    for i in 0..count {
        let doc_id = ValidatedDocumentId::from_uuid(Uuid::new_v4())?;
        let path = ValidatedPath::new(format!("/{test_type}/integrity_test_{i:04}.md"))?;
        let title = ValidatedTitle::new(format!("{test_type} Integrity Test Document {i}"))?;

        let content = format!(
            r#"# {} Integrity Test Document {}

This is a test document for data integrity validation.

## Document Details

- Test Type: {}
- Document ID: {}
- Document Number: {}
- Content Length: Variable

## Content Section

This section contains test data for integrity verification.
The content is designed to test various aspects of data integrity
including content validation, size consistency, and metadata accuracy.

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim
veniam, quis nostrud exercitation ullamco laboris.

## Validation Markers

- START_MARKER: integrity_test_{}
- END_MARKER: test_complete
- CHECKSUM_DATA: {}

This concludes the integrity test document.
"#,
            test_type,
            i,
            test_type,
            doc_id,
            i,
            i,
            format_args!("{:x}", i * 12345) // Simple checksum simulation
        )
        .into_bytes();

        let tags = vec![];

        let now = chrono::Utc::now();

        let content_size = content.len();
        let document = Document {
            id: doc_id,
            path,
            title,
            content,
            tags,
            created_at: now,
            updated_at: now,
            size: content_size,
            embedding: None,
        };

        documents.push(document);
    }

    Ok(documents)
}
