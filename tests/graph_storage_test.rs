//! Integration tests for dual storage architecture
//!
//! This test suite validates the graph storage implementation and ensures
//! it meets performance requirements for code intelligence features.

use anyhow::Result;
use kotadb::contracts::Storage;
use kotadb::graph_storage::{
    CompressionType, GraphEdge, GraphNode, GraphStorage, GraphStorageConfig, NodeLocation, SyncMode,
};
use kotadb::hybrid_storage::{HybridStorage, HybridStorageConfig};
use kotadb::native_graph_storage::NativeGraphStorage;
use kotadb::symbol_storage::RelationType;
use petgraph::Direction;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::sync::RwLock as TokioRwLock;
use uuid::Uuid;

/// Helper to create a test graph node
fn create_test_node(name: &str, node_type: &str) -> GraphNode {
    GraphNode {
        id: Uuid::new_v4(),
        node_type: node_type.to_string(),
        qualified_name: name.to_string(),
        file_path: format!("src/{}.rs", name.replace("::", "/")),
        location: NodeLocation {
            start_line: 10,
            start_column: 1,
            end_line: 20,
            end_column: 30,
        },
        metadata: HashMap::new(),
        updated_at: chrono::Utc::now().timestamp(),
    }
}

/// Helper to create a test edge
fn create_test_edge(relation_type: RelationType) -> GraphEdge {
    GraphEdge {
        relation_type,
        location: NodeLocation {
            start_line: 15,
            start_column: 5,
            end_line: 15,
            end_column: 25,
        },
        context: Some("test_function()".to_string()),
        metadata: HashMap::new(),
        created_at: chrono::Utc::now().timestamp(),
    }
}

#[tokio::test]
async fn test_native_graph_storage_basic_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig::default();

    let mut storage = NativeGraphStorage::new(temp_dir.path(), config).await?;

    // Test node operations
    let node1 = create_test_node("module::function1", "function");
    let node_id1 = node1.id;
    storage.store_node(node_id1, node1.clone()).await?;

    let retrieved = storage.get_node(node_id1).await?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().qualified_name, "module::function1");

    // Test edge operations
    let node2 = create_test_node("module::function2", "function");
    let node_id2 = node2.id;
    storage.store_node(node_id2, node2).await?;

    let edge = create_test_edge(RelationType::Calls);
    storage.store_edge(node_id1, node_id2, edge.clone()).await?;

    // Test edge retrieval
    let outgoing = storage.get_edges(node_id1, Direction::Outgoing).await?;
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].0, node_id2);

    let incoming = storage.get_edges(node_id2, Direction::Incoming).await?;
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].0, node_id1);

    Ok(())
}

#[tokio::test]
async fn test_graph_traversal_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig::default();

    let mut storage = NativeGraphStorage::new(temp_dir.path(), config).await?;

    // Create a graph with 100 nodes
    let mut node_ids = Vec::new();
    for i in 0..100 {
        let node = create_test_node(&format!("module::function{}", i), "function");
        let node_id = node.id;
        node_ids.push(node_id);
        storage.store_node(node_id, node).await?;
    }

    // Create edges forming a call chain
    for i in 0..99 {
        let edge = create_test_edge(RelationType::Calls);
        storage
            .store_edge(node_ids[i], node_ids[i + 1], edge)
            .await?;
    }

    // Test subgraph extraction performance
    let start = Instant::now();
    let subgraph = storage.get_subgraph(&[node_ids[0]], 5).await?;
    let elapsed = start.elapsed();

    // Should complete in under 10ms
    assert!(
        elapsed.as_millis() < 10,
        "Subgraph extraction took {}ms, expected <10ms",
        elapsed.as_millis()
    );
    assert_eq!(subgraph.nodes.len(), 6); // Root + 5 levels deep

    // Test path finding performance
    let start = Instant::now();
    let paths = storage.find_paths(node_ids[0], node_ids[10], 5).await?;
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 10,
        "Path finding took {}ms, expected <10ms",
        elapsed.as_millis()
    );
    assert!(!paths.is_empty());
    assert_eq!(paths[0].length, 11); // 0 -> 1 -> ... -> 10

    Ok(())
}

#[tokio::test]
async fn test_hybrid_storage_routing() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let config = HybridStorageConfig {
        enable_graph_storage: true,
        graph_patterns: vec!["/symbols/*".to_string(), "/relationships/*".to_string()],
        graph_config: GraphStorageConfig::default(),
        routing_cache_size: 100,
    };

    let mut storage = HybridStorage::new(temp_dir.path(), config).await?;

    // Test that symbol operations go to graph storage
    let symbol_node = create_test_node("test::Symbol", "class");
    let symbol_id = symbol_node.id;
    storage.store_node(symbol_id, symbol_node).await?;

    let retrieved = storage.get_node(symbol_id).await?;
    assert!(retrieved.is_some());

    // Test that relationships are stored in graph
    let node2 = create_test_node("test::Function", "function");
    let node2_id = node2.id;
    storage.store_node(node2_id, node2).await?;

    let edge = create_test_edge(RelationType::Calls);
    storage.store_edge(symbol_id, node2_id, edge).await?;

    let edges = storage.get_edges(symbol_id, Direction::Outgoing).await?;
    assert_eq!(edges.len(), 1);

    // Test statistics
    let stats = storage.get_stats().await?;
    assert_eq!(stats.graph_ops, 2); // Two store_node operations

    Ok(())
}

#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig::default();

    let mut storage = NativeGraphStorage::new(temp_dir.path(), config).await?;

    // Batch insert nodes
    let mut nodes = Vec::new();
    for i in 0..50 {
        let node = create_test_node(&format!("batch::function{}", i), "function");
        nodes.push((node.id, node));
    }

    let start = Instant::now();
    storage.batch_insert_nodes(nodes.clone()).await?;
    let elapsed = start.elapsed();

    println!("Batch insert 50 nodes: {}ms", elapsed.as_millis());
    assert!(elapsed.as_millis() < 100); // Should be fast

    // Verify all nodes were inserted
    for (id, _) in &nodes {
        assert!(storage.get_node(*id).await?.is_some());
    }

    // Batch insert edges
    let mut edges = Vec::new();
    for i in 0..49 {
        let edge = create_test_edge(RelationType::Calls);
        edges.push((nodes[i].0, nodes[i + 1].0, edge));
    }

    let start = Instant::now();
    storage.batch_insert_edges(edges).await?;
    let elapsed = start.elapsed();

    println!("Batch insert 49 edges: {}ms", elapsed.as_millis());
    assert!(elapsed.as_millis() < 100);

    // Verify graph statistics
    let stats = storage.get_graph_stats().await?;
    assert_eq!(stats.node_count, 50);
    assert_eq!(stats.edge_count, 49);

    Ok(())
}

#[tokio::test]
async fn test_complex_graph_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig::default();

    let mut storage = NativeGraphStorage::new(temp_dir.path(), config).await?;

    // Create a more complex graph structure
    // Module A with 3 functions
    let mod_a = create_test_node("module_a", "module");
    let fn_a1 = create_test_node("module_a::func1", "function");
    let fn_a2 = create_test_node("module_a::func2", "function");
    let fn_a3 = create_test_node("module_a::func3", "function");

    // Module B with 2 functions
    let mod_b = create_test_node("module_b", "module");
    let fn_b1 = create_test_node("module_b::func1", "function");
    let fn_b2 = create_test_node("module_b::func2", "function");

    // Store all nodes
    storage.store_node(mod_a.id, mod_a.clone()).await?;
    storage.store_node(fn_a1.id, fn_a1.clone()).await?;
    storage.store_node(fn_a2.id, fn_a2.clone()).await?;
    storage.store_node(fn_a3.id, fn_a3.clone()).await?;
    storage.store_node(mod_b.id, mod_b.clone()).await?;
    storage.store_node(fn_b1.id, fn_b1.clone()).await?;
    storage.store_node(fn_b2.id, fn_b2.clone()).await?;

    // Create relationships
    // Module contains functions
    storage
        .store_edge(mod_a.id, fn_a1.id, create_test_edge(RelationType::ChildOf))
        .await?;
    storage
        .store_edge(mod_a.id, fn_a2.id, create_test_edge(RelationType::ChildOf))
        .await?;
    storage
        .store_edge(mod_a.id, fn_a3.id, create_test_edge(RelationType::ChildOf))
        .await?;
    storage
        .store_edge(mod_b.id, fn_b1.id, create_test_edge(RelationType::ChildOf))
        .await?;
    storage
        .store_edge(mod_b.id, fn_b2.id, create_test_edge(RelationType::ChildOf))
        .await?;

    // Cross-module calls
    storage
        .store_edge(fn_a1.id, fn_b1.id, create_test_edge(RelationType::Calls))
        .await?;
    storage
        .store_edge(fn_a2.id, fn_b1.id, create_test_edge(RelationType::Calls))
        .await?;
    storage
        .store_edge(fn_b1.id, fn_b2.id, create_test_edge(RelationType::Calls))
        .await?;

    // Test getting nodes by type
    let functions = storage.get_nodes_by_type("function").await?;
    assert_eq!(functions.len(), 5);

    let modules = storage.get_nodes_by_type("module").await?;
    assert_eq!(modules.len(), 2);

    // Test subgraph extraction from module
    let subgraph = storage.get_subgraph(&[mod_a.id], 2).await?;
    assert_eq!(subgraph.nodes.len(), 4); // mod_a + 3 functions

    // Test finding call paths
    let paths = storage.find_paths(fn_a1.id, fn_b2.id, 10).await?;
    assert!(!paths.is_empty());
    assert_eq!(paths[0].length, 3); // fn_a1 -> fn_b1 -> fn_b2

    Ok(())
}

#[tokio::test]
async fn test_edge_metadata_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig::default();

    let mut storage = NativeGraphStorage::new(temp_dir.path(), config).await?;

    let node1 = create_test_node("test::node1", "function");
    let node2 = create_test_node("test::node2", "function");

    storage.store_node(node1.id, node1.clone()).await?;
    storage.store_node(node2.id, node2.clone()).await?;

    // Store edge with initial metadata
    let mut edge = create_test_edge(RelationType::Calls);
    edge.metadata
        .insert("frequency".to_string(), "10".to_string());
    storage.store_edge(node1.id, node2.id, edge).await?;

    // Update edge metadata
    let mut new_metadata = HashMap::new();
    new_metadata.insert("frequency".to_string(), "20".to_string());
    new_metadata.insert("last_called".to_string(), "2024-01-01".to_string());

    storage
        .update_edge_metadata(node1.id, node2.id, new_metadata.clone())
        .await?;

    // Verify metadata was updated
    let edges = storage.get_edges(node1.id, Direction::Outgoing).await?;
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].1.metadata.get("frequency").unwrap(), "20");
    assert_eq!(
        edges[0].1.metadata.get("last_called").unwrap(),
        "2024-01-01"
    );

    // Test edge removal
    let removed = storage.remove_edge(node1.id, node2.id).await?;
    assert!(removed);

    let edges = storage.get_edges(node1.id, Direction::Outgoing).await?;
    assert!(edges.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_persistence_and_recovery() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig {
        cache_size: 100,
        enable_wal: true,
        compression: CompressionType::Snappy,
        sync_mode: SyncMode::Normal,
        max_traversal_depth: 10,
        max_path_results: 100,
    };

    // Create and populate storage
    {
        let mut storage = NativeGraphStorage::new(temp_dir.path(), config.clone()).await?;

        // Insert test data
        for i in 0..10 {
            let node = create_test_node(&format!("persist::func{}", i), "function");
            storage.store_node(node.id, node.clone()).await?;

            if i > 0 {
                let prev_node = create_test_node(&format!("persist::func{}", i - 1), "function");
                storage
                    .store_edge(prev_node.id, node.id, create_test_edge(RelationType::Calls))
                    .await?;
            }
        }

        storage.sync().await?;
    }

    // Reopen storage and verify data persisted
    {
        let storage = NativeGraphStorage::new(temp_dir.path(), config).await?;

        // Verify nodes are still there
        for i in 0..10 {
            let node_name = format!("persist::func{}", i);
            let nodes = storage.get_nodes_by_type("function").await?;
            assert!(
                nodes.len() >= 10,
                "Expected at least 10 nodes after recovery"
            );
        }

        let stats = storage.get_graph_stats().await?;
        assert_eq!(stats.node_count, 10);
        // Note: Edges might not persist in this implementation yet
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = GraphStorageConfig::default();

    let storage = Arc::new(TokioRwLock::new(
        NativeGraphStorage::new(temp_dir.path(), config).await?,
    ));

    // Spawn multiple tasks doing concurrent operations
    let mut handles = Vec::new();

    for i in 0..5 {
        let storage = storage.clone();
        let handle = tokio::spawn(async move {
            let node = create_test_node(&format!("concurrent::task{}", i), "function");
            let mut storage = storage.write().await;
            storage.store_node(node.id, node).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await??;
    }

    // Verify all nodes were inserted
    let storage = storage.read().await;
    let functions = storage.get_nodes_by_type("function").await?;
    assert_eq!(functions.len(), 5);

    Ok(())
}
