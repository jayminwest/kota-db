//! Hybrid relationship query engine that integrates binary symbols with relationship queries
//!
//! This module provides the integration layer between the fast binary symbol format
//! and the relationship query functionality, ensuring sub-10ms query latency while
//! maintaining full API compatibility.

use anyhow::{Context, Result};
use std::cell::RefCell;
use std::path::Path;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::{
    binary_relationship_bridge::BinaryRelationshipBridge,
    binary_symbols::BinarySymbolReader,
    dependency_extractor::DependencyGraph,
    parsing::SymbolType,
    relationship_query::{
        RelationshipLocation, RelationshipMatch, RelationshipQueryConfig, RelationshipQueryResult,
        RelationshipQueryType, RelationshipStats,
    },
    types::RelationType,
};

/// Hybrid relationship query engine that uses binary symbols
pub struct HybridRelationshipEngine {
    /// Binary symbol reader for fast symbol lookup
    symbol_reader: Option<BinarySymbolReader>,
    /// Dependency graph built from relationships (using RefCell for interior mutability)
    dependency_graph: RefCell<Option<DependencyGraph>>,
    /// Database path for on-demand relationship extraction
    db_path: std::path::PathBuf,
    /// Configuration
    config: RelationshipQueryConfig,
}

impl HybridRelationshipEngine {
    /// Create a new hybrid engine from database paths
    #[instrument]
    pub async fn new(db_path: &Path, config: RelationshipQueryConfig) -> Result<Self> {
        info!("Initializing hybrid relationship engine");

        // Try to load binary symbols if available
        let symbol_db_path = db_path.join("symbols.kota");
        let symbol_reader = if symbol_db_path.exists() {
            info!("Loading binary symbol database from: {:?}", symbol_db_path);
            match BinarySymbolReader::open(&symbol_db_path) {
                Ok(reader) => {
                    info!("Loaded {} binary symbols", reader.symbol_count());
                    Some(reader)
                }
                Err(e) => {
                    warn!("Failed to load binary symbols: {}", e);
                    None
                }
            }
        } else {
            debug!("Binary symbol database not found at: {:?}", symbol_db_path);
            None
        };

        // Try to load dependency graph if available
        let graph_db_path = db_path.join("dependency_graph.bin");
        let dependency_graph = if graph_db_path.exists() {
            info!("Loading dependency graph from: {:?}", graph_db_path);
            match Self::load_dependency_graph(&graph_db_path) {
                Ok(graph) => {
                    info!(
                        "Loaded dependency graph with {} nodes",
                        graph.graph.node_count()
                    );
                    Some(graph)
                }
                Err(e) => {
                    warn!("Failed to load dependency graph: {}", e);
                    None
                }
            }
        } else {
            debug!("Dependency graph not found at: {:?}", graph_db_path);
            None
        };

        Ok(Self {
            symbol_reader,
            dependency_graph: RefCell::new(dependency_graph),
            db_path: db_path.to_path_buf(),
            config,
        })
    }

    /// Execute a relationship query using the hybrid approach
    #[instrument(skip(self))]
    pub async fn execute_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        info!("Executing relationship query: {:?}", query_type);
        let start = std::time::Instant::now();

        // First try binary symbols if available, even without dependency graph for basic queries
        let result = if self.symbol_reader.is_some() {
            debug!(
                "Using binary symbol path for query (dependency graph available: {})",
                self.dependency_graph.borrow().is_some()
            );
            self.execute_binary_query(query_type.clone()).await
        } else {
            debug!("Falling back to legacy symbol storage path");
            self.execute_legacy_query(query_type.clone()).await
        };

        let elapsed = start.elapsed();

        match &result {
            Ok(r) => {
                info!(
                    "Query completed in {:?} - found {} direct, {} indirect relationships",
                    elapsed, r.stats.direct_count, r.stats.indirect_count
                );
                if elapsed.as_millis() > 10 {
                    warn!("Query exceeded 10ms target: {:?}", elapsed);
                }
            }
            Err(e) => {
                warn!("Query failed after {:?}: {}", elapsed, e);
            }
        }

        result
    }

    /// Execute query using binary symbols and optional dependency graph
    async fn execute_binary_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        let reader = self
            .symbol_reader
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Binary symbol reader not available"))?;

        match query_type.clone() {
            RelationshipQueryType::FindCallers { target } => {
                // Ensure dependency graph is available, extracting on-demand if needed
                if let Err(e) = self.ensure_dependency_graph("find-callers query").await {
                    return Ok(RelationshipQueryResult {
                        query_type,
                        direct_relationships: vec![],
                        indirect_relationships: vec![],
                        stats: RelationshipStats {
                            direct_count: 0,
                            indirect_count: 0,
                            symbols_analyzed: reader.symbol_count(),
                            execution_time_ms: 0,
                            truncated: false,
                        },
                        summary: format!(
                            "Symbol '{}' found in binary database (total {} symbols loaded), but on-demand relationship extraction failed: {}. \
                            Consider re-running ingest-repo with relationship extraction enabled for better performance.",
                            target, reader.symbol_count(), e
                        ),
                    });
                }

                // Now we should have a graph, borrow it
                let graph_ref = self.dependency_graph.borrow();
                let graph = graph_ref.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Dependency graph unavailable after extraction attempt")
                })?;

                // Look up target symbol by name
                let (_symbol, target_id) = reader
                    .find_symbol_by_name(&target)
                    .ok_or_else(|| anyhow::anyhow!("Symbol '{}' not found", target))?;

                // Find all callers in the dependency graph
                let callers = graph.find_dependents(target_id);

                // Convert to relationship matches
                let mut direct_relationships = Vec::new();
                for (caller_id, relation_type) in callers.iter() {
                    if let Some(symbol) = reader.find_symbol(*caller_id) {
                        let symbol_name = reader.get_symbol_name(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get symbol name for UUID {}: {}", caller_id, e);
                            format!("symbol_{}", caller_id)
                        });
                        let file_path = reader.get_symbol_file_path(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get file path for symbol: {}", e);
                            "unknown".to_string()
                        });

                        direct_relationships.push(RelationshipMatch {
                            symbol_id: Uuid::from_bytes(symbol.id),
                            symbol_name: symbol_name.clone(),
                            qualified_name: format!("{}::{}", file_path, symbol_name),
                            symbol_type: Self::convert_symbol_type(symbol.kind),
                            file_path: file_path.clone(),
                            relation_type: relation_type.clone(),
                            location: RelationshipLocation {
                                line_number: symbol.start_line as usize,
                                column_number: 0,
                                file_path: file_path.clone(),
                            },
                            context: format!("Calls {} at line {}", target, symbol.start_line),
                        });
                    }
                }

                Ok(RelationshipQueryResult {
                    query_type,
                    direct_relationships,
                    indirect_relationships: vec![],
                    stats: RelationshipStats {
                        direct_count: callers.len(),
                        indirect_count: 0,
                        symbols_analyzed: reader.symbol_count(),
                        execution_time_ms: 0,
                        truncated: false,
                    },
                    summary: format!("Found {} direct callers of '{}'", callers.len(), target),
                })
            }
            RelationshipQueryType::ImpactAnalysis { target } => {
                // Ensure dependency graph is available, extracting on-demand if needed
                if let Err(e) = self.ensure_dependency_graph("impact analysis").await {
                    return Ok(RelationshipQueryResult {
                        query_type,
                        direct_relationships: vec![],
                        indirect_relationships: vec![],
                        stats: RelationshipStats {
                            direct_count: 0,
                            indirect_count: 0,
                            symbols_analyzed: reader.symbol_count(),
                            execution_time_ms: 0,
                            truncated: false,
                        },
                        summary: format!(
                            "Symbol '{}' found in binary database (total {} symbols loaded), but on-demand relationship extraction failed: {}. \
                            Consider re-running ingest-repo with relationship extraction enabled for better performance.",
                            target, reader.symbol_count(), e
                        ),
                    });
                }

                // Now we should have a graph, borrow it
                let graph_ref = self.dependency_graph.borrow();
                let graph = graph_ref.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Dependency graph unavailable after extraction attempt")
                })?;

                // For impact analysis, find all transitive dependencies
                let (_symbol, target_id) = reader
                    .find_symbol_by_name(&target)
                    .ok_or_else(|| anyhow::anyhow!("Symbol '{}' not found", target))?;

                let impacted =
                    self.find_transitive_dependents(graph, target_id, self.config.max_depth);

                // Convert to relationship matches
                let mut direct_relationships = Vec::new();
                for (id, relation_type) in impacted.iter() {
                    if let Some(symbol) = reader.find_symbol(*id) {
                        let symbol_name = reader.get_symbol_name(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get symbol name for UUID {}: {}", id, e);
                            format!("symbol_{}", id)
                        });
                        let file_path = reader.get_symbol_file_path(&symbol).unwrap_or_else(|e| {
                            warn!("Failed to get file path for symbol: {}", e);
                            "unknown".to_string()
                        });

                        direct_relationships.push(RelationshipMatch {
                            symbol_id: Uuid::from_bytes(symbol.id),
                            symbol_name: symbol_name.clone(),
                            qualified_name: format!("{}::{}", file_path, symbol_name),
                            symbol_type: Self::convert_symbol_type(symbol.kind),
                            file_path: file_path.clone(),
                            relation_type: relation_type.clone(),
                            location: RelationshipLocation {
                                line_number: symbol.start_line as usize,
                                column_number: 0,
                                file_path: file_path.clone(),
                            },
                            context: format!("Would be impacted by changes to {}", target),
                        });
                    }
                }

                Ok(RelationshipQueryResult {
                    query_type,
                    direct_relationships,
                    indirect_relationships: vec![],
                    stats: RelationshipStats {
                        direct_count: impacted.len(),
                        indirect_count: 0,
                        symbols_analyzed: reader.symbol_count(),
                        execution_time_ms: 0,
                        truncated: false,
                    },
                    summary: format!(
                        "{} symbols would be impacted by changes to '{}'",
                        impacted.len(),
                        target
                    ),
                })
            }
            _ => {
                // For other query types, fall back to legacy implementation
                self.execute_legacy_query(query_type).await
            }
        }
    }

    /// Execute query using legacy symbol storage
    async fn execute_legacy_query(
        &self,
        query_type: RelationshipQueryType,
    ) -> Result<RelationshipQueryResult> {
        // For now, return an error indicating the need for proper setup
        // In a future version, we could build a dependency graph from symbol storage here
        Err(anyhow::anyhow!(
            "Legacy relationship queries require both binary symbols and dependency graph. \
            Please ensure the repository was ingested with symbol and relationship extraction enabled."
        ))
    }

    /// Find all symbols that transitively depend on the given symbol
    fn find_transitive_dependents(
        &self,
        graph: &DependencyGraph,
        target_id: Uuid,
        max_depth: usize,
    ) -> Vec<(Uuid, RelationType)> {
        use std::collections::{HashSet, VecDeque};

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct dependents
        queue.push_back((target_id, 0));
        visited.insert(target_id);

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let dependents = graph.find_dependents(current_id);
            for (dependent_id, relation_type) in dependents {
                if !visited.contains(&dependent_id) {
                    visited.insert(dependent_id);
                    result.push((dependent_id, relation_type.clone()));
                    queue.push_back((dependent_id, depth + 1));
                }
            }
        }

        result
    }

    /// Save dependency graph to binary file (async version)
    pub async fn save_dependency_graph_async(
        graph: &DependencyGraph,
        path: &Path,
    ) -> Result<()> {
        info!(
            "Saving dependency graph with {} nodes to: {:?}",
            graph.graph.node_count(),
            path
        );

        let path = path.to_path_buf();
        let serializable = graph.to_serializable();

        // Use spawn_blocking to handle the blocking serialization operation
        tokio::task::spawn_blocking(move || -> Result<()> {
            use std::fs::File;
            use std::io::BufWriter;

            // Create parent directory if it doesn't exist
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            let file = File::create(&path)
                .with_context(|| format!("Failed to create dependency graph file: {:?}", path))?;
            let writer = BufWriter::new(file);

            // Serialize using bincode for efficiency
            bincode::serialize_into(writer, &serializable)
                .context("Failed to serialize dependency graph")?;

            info!("Successfully saved dependency graph to: {:?}", path);
            Ok(())
        })
        .await
        .context("Task join error")?
    }

    /// Save dependency graph to binary file (legacy sync version for backward compatibility)
    pub fn save_dependency_graph(graph: &DependencyGraph, path: &Path) -> Result<()> {
        use std::fs::File;
        use std::io::BufWriter;

        info!(
            "Saving dependency graph with {} nodes to: {:?}",
            graph.graph.node_count(),
            path
        );

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let file = File::create(path)
            .with_context(|| format!("Failed to create dependency graph file: {:?}", path))?;
        let writer = BufWriter::new(file);

        // Convert to serializable format
        let serializable = graph.to_serializable();

        // Serialize using bincode for efficiency
        bincode::serialize_into(writer, &serializable)
            .context("Failed to serialize dependency graph")?;

        info!("Successfully saved dependency graph to: {:?}", path);
        Ok(())
    }

    /// Load dependency graph from binary file
    fn load_dependency_graph(path: &Path) -> Result<DependencyGraph> {
        use std::fs::File;
        use std::io::BufReader;

        debug!("Loading dependency graph from: {:?}", path);

        let file = File::open(path)
            .with_context(|| format!("Failed to open dependency graph file: {:?}", path))?;
        let reader = BufReader::new(file);

        // Deserialize using bincode for efficiency
        let serializable: crate::dependency_extractor::SerializableDependencyGraph =
            bincode::deserialize_from(reader).context("Failed to deserialize dependency graph")?;

        // Convert from serializable format
        DependencyGraph::from_serializable(serializable)
            .context("Failed to reconstruct dependency graph from serialized data")
    }

    /// Convert binary symbol kind to SymbolType
    fn convert_symbol_type(kind: u8) -> SymbolType {
        match kind {
            1 => SymbolType::Function,
            2 => SymbolType::Method,
            3 => SymbolType::Class,
            4 => SymbolType::Struct,
            5 => SymbolType::Enum,
            6 => SymbolType::Variable,
            7 => SymbolType::Constant,
            8 => SymbolType::Module,
            _ => SymbolType::Other("Unknown".to_string()),
        }
    }

    /// Ensure dependency graph is available, extracting on-demand if necessary
    #[instrument(skip(self))]
    async fn ensure_dependency_graph(&self, query_context: &str) -> Result<()> {
        let has_graph = self.dependency_graph.borrow().is_some();
        if has_graph {
            return Ok(());
        }

        info!("Dependency graph not cached, attempting on-demand extraction for {}", query_context);

        match self.extract_relationships_on_demand().await {
            Ok(extracted_graph) => {
                info!(
                    "Successfully extracted relationships on-demand with {} nodes",
                    extracted_graph.graph.node_count()
                );

                // Store the extracted graph for future queries
                *self.dependency_graph.borrow_mut() = Some(extracted_graph);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to extract relationships on-demand: {}", e);
                Err(e)
            }
        }
    }

    /// Extract relationships on-demand from binary symbols and source files
    /// This method bridges the gap when binary symbols exist but dependency graph is missing
    #[instrument(skip(self))]
    async fn extract_relationships_on_demand(&self) -> Result<DependencyGraph> {
        info!("Starting on-demand relationship extraction from binary symbols");
        let start = std::time::Instant::now();

        let symbol_db_path = self.db_path.join("symbols.kota");

        // Ensure we have binary symbols available
        if !symbol_db_path.exists() {
            return Err(anyhow::anyhow!(
                "Binary symbol database not found at: {:?}",
                symbol_db_path
            ));
        }

        // Get the storage path to access source files
        let storage_path = self.db_path.join("storage");
        if !storage_path.exists() {
            return Err(anyhow::anyhow!(
                "Storage path not found at: {:?}",
                storage_path
            ));
        }

        // Find source files from the storage directory
        let files = self.collect_source_files(&storage_path).await?;
        info!(
            "Collected {} source files for relationship extraction",
            files.len()
        );

        if files.is_empty() {
            return Err(anyhow::anyhow!(
                "No source files found for relationship extraction"
            ));
        }

        // Create relationship bridge and extract relationships
        let bridge = BinaryRelationshipBridge::new();
        let dependency_graph = bridge
            .extract_relationships(&symbol_db_path, &self.db_path, &files)
            .with_context(|| "Failed to extract relationships from binary symbols")?;

        let elapsed = start.elapsed();
        info!(
            "On-demand relationship extraction completed in {:?}, extracted {} relationships",
            elapsed,
            dependency_graph.graph.node_count()
        );

        // Save the extracted graph for future use
        let graph_path = self.db_path.join("dependency_graph.bin");
        if let Err(e) = Self::save_dependency_graph_async(&dependency_graph, &graph_path).await {
            warn!("Failed to cache extracted dependency graph: {}", e);
        } else {
            info!("Cached extracted dependency graph to: {:?}", graph_path);
        }

        Ok(dependency_graph)
    }

    /// Collect source files from the storage directory for relationship extraction
    async fn collect_source_files(
        &self,
        storage_path: &Path,
    ) -> Result<Vec<(std::path::PathBuf, Vec<u8>)>> {
        use tokio::fs;

        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit for security
        let mut files = Vec::new();
        let mut entries = fs::read_dir(storage_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process source code files (Rust, Python, JavaScript, TypeScript, etc.)
            if let Some(extension) = path.extension() {
                let ext = extension.to_string_lossy().to_lowercase();
                if matches!(
                    ext.as_str(),
                    "rs" | "py" | "js" | "ts" | "cpp" | "c" | "h" | "hpp" | "java" | "go" | "rb"
                ) {
                    // Check file size before reading
                    match fs::metadata(&path).await {
                        Ok(metadata) => {
                            if metadata.len() > MAX_FILE_SIZE {
                                warn!(
                                    "Skipping file {} - size {} bytes exceeds limit {} bytes",
                                    path.display(),
                                    metadata.len(),
                                    MAX_FILE_SIZE
                                );
                                continue;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to get metadata for file {}: {}", path.display(), e);
                            continue;
                        }
                    }

                    // Read file contents
                    match fs::read(&path).await {
                        Ok(contents) => {
                            files.push((path, contents));
                        }
                        Err(e) => {
                            warn!("Failed to read file {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    /// Get statistics about the hybrid engine
    pub fn get_stats(&self) -> HybridEngineStats {
        let graph_borrowed = self.dependency_graph.borrow();
        HybridEngineStats {
            binary_symbols_loaded: self
                .symbol_reader
                .as_ref()
                .map(|r| r.symbol_count())
                .unwrap_or(0),
            graph_nodes_loaded: graph_borrowed
                .as_ref()
                .map(|g| g.graph.node_count())
                .unwrap_or(0),
            using_binary_path: self.symbol_reader.is_some() && graph_borrowed.is_some(),
        }
    }
}

/// Statistics about the hybrid engine
#[derive(Debug, Clone)]
pub struct HybridEngineStats {
    pub binary_symbols_loaded: usize,
    pub graph_nodes_loaded: usize,
    pub using_binary_path: bool,
}
