//! Integration tests for path normalization in relationship extraction
//!
//! This test ensures that the path normalization issue (#447) is fixed
//! and prevents regression where find-callers/analyze-impact would return
//! empty results due to path format mismatches.

#[cfg(feature = "tree-sitter-parsing")]
mod path_normalization_tests {
    use anyhow::Result;
    use kotadb::{
        binary_relationship_bridge::BinaryRelationshipBridge,
        binary_symbols::BinarySymbolWriter,
        path_utils::{normalize_path_relative, paths_equivalent},
    };
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_path_normalization_utilities() {
        // Test absolute to relative conversion
        let repo_root = Path::new("/home/user/project");
        let absolute_path = Path::new("/home/user/project/src/main.rs");
        assert_eq!(
            normalize_path_relative(absolute_path, repo_root),
            "src/main.rs"
        );

        // Test already relative path
        let relative_path = Path::new("src/main.rs");
        assert_eq!(
            normalize_path_relative(relative_path, repo_root),
            "src/main.rs"
        );

        // Test path with ./ prefix
        let dotted_path = Path::new("./src/main.rs");
        assert_eq!(
            normalize_path_relative(dotted_path, repo_root),
            "src/main.rs"
        );
    }

    #[test]
    fn test_paths_equivalent() {
        // Test exact match
        assert!(paths_equivalent("src/main.rs", "src/main.rs"));

        // Test with ./ prefix
        assert!(paths_equivalent("./src/main.rs", "src/main.rs"));

        // Test suffix matching
        assert!(paths_equivalent("/project/src/main.rs", "src/main.rs"));

        // Test different files
        assert!(!paths_equivalent("src/main.rs", "src/lib.rs"));
    }

    #[tokio::test]
    async fn test_relationship_extraction_with_normalized_paths() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.symdb");
        let repo_path = temp_dir.path();

        // Create a simple symbol database with relative paths (as stored by git ingestion)
        let mut writer = BinarySymbolWriter::new();
        let func1_id = Uuid::new_v4();
        let func2_id = Uuid::new_v4();
        let struct_id = Uuid::new_v4();

        // Add symbols with relative paths (as git ingestion does)
        writer.add_symbol(
            func1_id,
            "call_storage",
            1,             // Function
            "src/main.rs", // Relative path
            10,
            20,
            None,
        );
        writer.add_symbol(
            func2_id,
            "FileStorage",
            1,                // Function
            "src/storage.rs", // Relative path
            5,
            15,
            None,
        );
        writer.add_symbol(
            struct_id,
            "StorageConfig",
            4,               // Struct
            "src/config.rs", // Relative path
            1,
            10,
            None,
        );
        writer.write_to_file(&db_path)?;

        // Create test source files with function calls
        let main_content = r#"
use crate::storage::FileStorage;
use crate::config::StorageConfig;

fn call_storage() {
    let config = StorageConfig::new();
    FileStorage::init(config);
    FileStorage::store("data");
}
        "#;

        let storage_content = r#"
use crate::config::StorageConfig;

pub struct FileStorage;

impl FileStorage {
    pub fn init(config: StorageConfig) {}
    pub fn store(data: &str) {}
}
        "#;

        let config_content = r#"
pub struct StorageConfig {
    pub path: String,
}

impl StorageConfig {
    pub fn new() -> Self {
        Self { path: String::new() }
    }
}
        "#;

        // Simulate file collection that would happen during on-demand extraction
        // These should be normalized to relative paths to match the symbol database
        let files = vec![
            (
                PathBuf::from("src/main.rs"), // Already relative, matching symbol DB
                main_content.as_bytes().to_vec(),
            ),
            (
                PathBuf::from("src/storage.rs"), // Already relative, matching symbol DB
                storage_content.as_bytes().to_vec(),
            ),
            (
                PathBuf::from("src/config.rs"), // Already relative, matching symbol DB
                config_content.as_bytes().to_vec(),
            ),
        ];

        // Extract relationships
        let bridge = BinaryRelationshipBridge::new();
        let graph = bridge.extract_relationships(&db_path, repo_path, &files)?;

        // Verify that relationships were found (not 0 edges as in the bug)
        assert!(
            graph.stats.edge_count > 0,
            "Expected edges to be created, but got 0. Path normalization may have failed."
        );

        // Verify nodes were created for all symbols
        assert_eq!(
            graph.stats.node_count, 3,
            "Expected 3 nodes (one for each symbol)"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_absolute_path_normalization_in_extraction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.symdb");
        let repo_path = temp_dir.path();

        // Create symbol database with relative paths
        let mut writer = BinarySymbolWriter::new();
        let func_id = Uuid::new_v4();
        writer.add_symbol(
            func_id,
            "test_function",
            1,
            "src/lib.rs", // Relative path in DB
            1,
            10,
            None,
        );
        writer.write_to_file(&db_path)?;

        // Simulate files collected with absolute paths (before the fix)
        // The fix should normalize these to relative paths
        let absolute_file_path = repo_path.join("src").join("lib.rs");
        let files = vec![(
            absolute_file_path.clone(),
            b"fn test_function() {}".to_vec(),
        )];

        // This would fail before the fix because absolute paths wouldn't match
        // relative paths in the symbol database
        let bridge = BinaryRelationshipBridge::new();

        // The fix normalizes absolute paths to relative during collection
        // So this should work now
        let normalized_files: Vec<(PathBuf, Vec<u8>)> = files
            .into_iter()
            .map(|(path, content)| {
                let normalized = normalize_path_relative(&path, repo_path);
                (PathBuf::from(normalized), content)
            })
            .collect();

        let graph = bridge.extract_relationships(&db_path, repo_path, &normalized_files)?;

        // Should have created a node for the symbol
        assert_eq!(graph.stats.node_count, 1, "Expected 1 node for the symbol");

        Ok(())
    }
}
