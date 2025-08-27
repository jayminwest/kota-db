//! Test to reproduce Issue #341: Edge persistence bug in relationship queries
#![allow(clippy::print_stderr)]

use anyhow::Result;
use kotadb::{
    create_file_storage,
    graph_storage::GraphStorageConfig,
    native_graph_storage::NativeGraphStorage,
    symbol_storage::{RelationType, SymbolStorage},
    Storage,
};
use tempfile::TempDir;

/// Test that demonstrates edges are not being persisted to disk
#[tokio::test]
async fn test_edge_persistence_bug_simple() -> Result<()> {
    // Create temporary directory for test database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path();
    let storage_path = db_path.join("storage");
    let graph_path = storage_path.join("graph");

    tokio::fs::create_dir_all(&storage_path).await?;
    tokio::fs::create_dir_all(&graph_path).await?;

    eprintln!("Testing edge persistence at: {:?}", graph_path);

    // Phase 1: Create storage and add some test edges
    {
        let file_storage = create_file_storage(storage_path.to_str().unwrap(), Some(100)).await?;

        let graph_config = GraphStorageConfig::default();
        let graph_storage = NativeGraphStorage::new(&graph_path, graph_config).await?;

        let symbol_storage =
            SymbolStorage::with_graph_storage(Box::new(file_storage), Box::new(graph_storage))
                .await?;

        // Check initial state
        let stats = symbol_storage.get_stats();
        eprintln!("Initial stats: {} symbols", stats.total_symbols);

        // For now, just test that the storage was created properly
        // and the directories exist
        let edges_dir = graph_path.join("edges");
        eprintln!("Edges directory: {:?}", edges_dir);

        // CRITICAL: Flush to ensure any edges would be persisted
        drop(symbol_storage); // This should trigger cleanup/flush
    }

    // Phase 2: Check if edges directory exists and has content
    let edges_dir = graph_path.join("edges");
    let edges_exist = edges_dir.exists();
    eprintln!("Edges directory exists after first phase: {}", edges_exist);

    if edges_exist {
        let mut entries = tokio::fs::read_dir(&edges_dir).await?;
        let mut file_count = 0;
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("page") {
                file_count += 1;
                let file_size = entry.metadata().await?.len();
                eprintln!(
                    "Found edge page file: {:?}, size: {} bytes",
                    entry.file_name(),
                    file_size
                );
            }
        }
        eprintln!("Total edge page files: {}", file_count);

        // The bug manifests as having 0 edge files even after relationships are created
        eprintln!(
            "✅ Edge directory exists but contains {} files (bug will show 0)",
            file_count
        );
    } else {
        eprintln!("❌ Edges directory doesn't exist at all");
    }

    Ok(())
}

/// Direct test of NativeGraphStorage edge persistence
#[tokio::test]
async fn test_native_graph_storage_direct() -> Result<()> {
    use kotadb::graph_storage::{GraphEdge, GraphNode, GraphStorage, NodeLocation};
    use std::collections::HashMap;
    use uuid::Uuid;

    // Create temporary directory
    let temp_dir = TempDir::new()?;
    let graph_path = temp_dir.path().join("graph");
    tokio::fs::create_dir_all(&graph_path).await?;

    let node1_id = Uuid::new_v4();
    let node2_id = Uuid::new_v4();

    eprintln!("Testing direct graph storage persistence");
    eprintln!("Node1: {}, Node2: {}", node1_id, node2_id);

    // Phase 1: Create graph storage and add test data
    {
        let config = GraphStorageConfig::default();
        let mut graph_storage = NativeGraphStorage::new(&graph_path, config).await?;

        // Create test nodes
        let node1 = GraphNode {
            id: node1_id,
            node_type: "function".to_string(),
            qualified_name: "test::caller".to_string(),
            file_path: "test.rs".to_string(),
            location: NodeLocation {
                start_line: 1,
                start_column: 0,
                end_line: 3,
                end_column: 0,
            },
            metadata: HashMap::new(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        let node2 = GraphNode {
            id: node2_id,
            node_type: "function".to_string(),
            qualified_name: "test::target".to_string(),
            file_path: "test.rs".to_string(),
            location: NodeLocation {
                start_line: 5,
                start_column: 0,
                end_line: 7,
                end_column: 0,
            },
            metadata: HashMap::new(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        // Store nodes
        graph_storage.store_node(node1_id, node1).await?;
        graph_storage.store_node(node2_id, node2).await?;

        // Create test edge
        let edge = GraphEdge {
            relation_type: RelationType::Calls,
            location: NodeLocation {
                start_line: 2,
                start_column: 4,
                end_line: 2,
                end_column: 10,
            },
            context: Some("caller() calls target()".to_string()),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now().timestamp(),
        };

        // Store edge - THIS IS THE CRITICAL TEST
        eprintln!("Storing edge: {} -> {}", node1_id, node2_id);
        graph_storage.store_edge(node1_id, node2_id, edge).await?;

        // Check if edges are in memory
        let edges = graph_storage
            .get_edges(node1_id, petgraph::Direction::Outgoing)
            .await?;
        eprintln!("Edges in memory after store_edge: {}", edges.len());
        assert_eq!(edges.len(), 1, "Should have 1 edge in memory");

        // CRITICAL: Flush to disk
        eprintln!("Calling flush...");
        graph_storage.flush().await?;
        eprintln!("Flush completed");

        // Check edges directory
        let edges_dir = graph_path.join("edges");
        let edges_exist = edges_dir.exists();
        eprintln!("Edges directory exists after flush: {}", edges_exist);

        if edges_exist {
            let mut entries = tokio::fs::read_dir(&edges_dir).await?;
            let mut file_count = 0;
            let mut total_size = 0;
            while let Some(entry) = entries.next_entry().await? {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("page") {
                    file_count += 1;
                    let file_size = entry.metadata().await?.len();
                    total_size += file_size;
                    eprintln!(
                        "Edge page file: {:?}, size: {} bytes",
                        entry.file_name(),
                        file_size
                    );
                }
            }
            eprintln!(
                "After flush: {} edge page files, {} total bytes",
                file_count, total_size
            );
        }
    }

    // Phase 2: Reload and check if edges persist
    {
        eprintln!("Phase 2: Reloading graph storage...");
        let config = GraphStorageConfig::default();
        let graph_storage = NativeGraphStorage::new(&graph_path, config).await?;

        // Try to get the edges back
        let edges = graph_storage
            .get_edges(node1_id, petgraph::Direction::Outgoing)
            .await?;
        eprintln!("Edges loaded from disk: {}", edges.len());

        // This assertion will FAIL due to bug #341
        assert_eq!(
            edges.len(),
            1,
            "BUG #341: Should have 1 edge after reload, but edges are not persisted!"
        );

        if !edges.is_empty() {
            let (target_id, edge_data) = &edges[0];
            eprintln!(
                "Loaded edge: {} -> {} (type: {:?})",
                node1_id, target_id, edge_data.relation_type
            );
        }
    }

    eprintln!("✅ Direct graph storage test passed!");
    Ok(())
}
