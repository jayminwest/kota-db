// AnalysisService - Unified analysis functionality for CLI, MCP, and API interfaces
//
// This service extracts code intelligence and relationship analysis logic from main.rs
// to enable feature parity across all KotaDB interfaces while maintaining identical behavior.

use anyhow::Result;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    binary_relationship_engine::BinaryRelationshipEngine,
    binary_symbols::BinarySymbolReader,
    dependency_extractor::SerializableDependencyGraph,
    path_utils::{detect_language_from_extension, is_potential_entry_point, is_test_file},
    relationship_query::{RelationshipQueryConfig, RelationshipQueryType},
};

// Simple database access trait for AnalysisService - only needs storage
pub trait AnalysisServiceDatabase: Send + Sync {
    fn storage(&self) -> Arc<Mutex<dyn crate::contracts::Storage>>;
}

/// Configuration options for find-callers analysis
#[derive(Debug, Clone, Default)]
pub struct CallersOptions {
    pub target: String,
    pub limit: Option<usize>,
    pub quiet: bool,
}

/// Configuration options for impact analysis
#[derive(Debug, Clone, Default)]
pub struct ImpactOptions {
    pub target: String,
    pub limit: Option<usize>,
    pub quiet: bool,
}

/// Configuration options for codebase overview
#[derive(Debug, Clone, serde::Serialize)]
pub struct OverviewOptions {
    pub format: String,
    pub top_symbols_limit: usize,
    pub entry_points_limit: usize,
    pub quiet: bool,
}

impl Default for OverviewOptions {
    fn default() -> Self {
        Self {
            format: "human".to_string(),
            top_symbols_limit: 10,
            entry_points_limit: 20,
            quiet: false,
        }
    }
}

/// Result structure for callers analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct CallersResult {
    pub callers: Vec<CallSite>,
    pub markdown: String,
    pub total_count: usize,
}

/// Result structure for impact analysis
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImpactResult {
    pub impacts: Vec<ImpactSite>,
    pub markdown: String,
    pub total_count: usize,
}

/// Result structure for codebase overview
#[derive(Debug, Clone, serde::Serialize)]
pub struct OverviewResult {
    pub overview_data: HashMap<String, serde_json::Value>,
    pub formatted_output: String,
}

/// Individual call site information
#[derive(Debug, Clone, serde::Serialize)]
pub struct CallSite {
    pub caller: String,
    pub file_path: String,
    pub line_number: Option<u32>,
    pub context: String,
}

/// Individual impact site information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImpactSite {
    pub affected_symbol: String,
    pub file_path: String,
    pub line_number: Option<u32>,
    pub impact_type: String,
}

/// Unified analysis service that handles relationship queries and codebase intelligence
pub struct AnalysisService<'a> {
    database: &'a dyn AnalysisServiceDatabase,
    db_path: PathBuf,
    relationship_engine: Option<BinaryRelationshipEngine>,
}

impl<'a> AnalysisService<'a> {
    /// Create a new AnalysisService instance
    pub fn new(database: &'a dyn AnalysisServiceDatabase, db_path: PathBuf) -> Self {
        Self {
            database,
            db_path,
            relationship_engine: None,
        }
    }

    /// Create or get the relationship engine, initializing if needed
    async fn get_relationship_engine(&mut self) -> Result<&BinaryRelationshipEngine> {
        if self.relationship_engine.is_none() {
            let engine = self.create_relationship_engine().await?;
            self.relationship_engine = Some(engine);
        }
        Ok(self.relationship_engine.as_ref().unwrap())
    }

    /// Create binary relationship engine with direct binary symbol access
    async fn create_relationship_engine(&self) -> Result<BinaryRelationshipEngine> {
        let config = RelationshipQueryConfig::default();
        let binary_engine = BinaryRelationshipEngine::new(&self.db_path, config).await?;

        // Check if we have any symbols or relationships loaded
        let stats = binary_engine.get_stats();
        if !stats.using_binary_path && stats.binary_symbols_loaded == 0 {
            return Err(anyhow::anyhow!(
                "No symbols found in database. Required steps:\n\
                 1. Index a codebase: kotadb index-codebase /path/to/repo\n\
                 2. Verify indexing: kotadb symbol-stats\n\
                 3. Then retry this command"
            ));
        }

        Ok(binary_engine)
    }

    /// Find callers of a specific symbol using the same logic as CLI FindCallers command
    pub async fn find_callers(&mut self, options: CallersOptions) -> Result<CallersResult> {
        let engine = self.get_relationship_engine().await?;
        let query_type = RelationshipQueryType::FindCallers {
            target: options.target.clone(),
        };

        let mut result = engine.execute_query(query_type).await?;

        // Apply limit if specified (0 means unlimited)
        if let Some(limit_value) = options.limit {
            if limit_value > 0 {
                result.limit_results(limit_value);
            }
        }

        let markdown = result.to_markdown();

        // Extract call sites (this would need to be implemented based on actual result structure)
        let callers = Vec::new(); // TODO: Parse from result
        let total_count = callers.len();

        Ok(CallersResult {
            callers,
            markdown,
            total_count,
        })
    }

    /// Analyze impact of changes to a specific symbol using CLI AnalyzeImpact logic
    pub async fn analyze_impact(&mut self, options: ImpactOptions) -> Result<ImpactResult> {
        let engine = self.get_relationship_engine().await?;
        let query_type = RelationshipQueryType::ImpactAnalysis {
            target: options.target.clone(),
        };

        let mut result = engine.execute_query(query_type).await?;

        // Apply limit if specified (0 means unlimited)
        if let Some(limit_value) = options.limit {
            if limit_value > 0 {
                result.limit_results(limit_value);
            }
        }

        let markdown = result.to_markdown();

        // Extract impact sites (this would need to be implemented based on actual result structure)
        let impacts = Vec::new(); // TODO: Parse from result
        let total_count = impacts.len();

        Ok(ImpactResult {
            impacts,
            markdown,
            total_count,
        })
    }

    /// Generate comprehensive codebase overview using the same logic as CLI CodebaseOverview
    pub async fn generate_overview(&self, options: OverviewOptions) -> Result<OverviewResult> {
        let mut overview_data = HashMap::new();

        // 1. Basic scale metrics from database
        let storage_arc = self.database.storage();
        let storage = storage_arc.lock().await;
        let all_docs = storage.list_all().await?;
        let doc_count = all_docs.len();
        let total_size: usize = all_docs.iter().map(|d| d.size).sum();

        overview_data.insert("total_files".to_string(), json!(doc_count));
        overview_data.insert("total_size_bytes".to_string(), json!(total_size));

        // 2. Symbol analysis (if available)
        let symbol_db_path = self.db_path.join("symbols.kota");
        let mut symbols_by_type: HashMap<String, usize> = HashMap::new();
        let mut symbols_by_language: HashMap<String, usize> = HashMap::new();
        let mut unique_files = HashSet::new();
        let mut total_symbols = 0;

        if symbol_db_path.exists() {
            match BinarySymbolReader::open(&symbol_db_path) {
                Ok(reader) => {
                    total_symbols = reader.symbol_count();

                    for symbol in reader.iter_symbols() {
                        // Count by type
                        let type_name = match crate::parsing::SymbolType::try_from(symbol.kind) {
                            Ok(symbol_type) => format!("{}", symbol_type),
                            Err(_) => format!("unknown({})", symbol.kind),
                        };
                        *symbols_by_type.entry(type_name).or_insert(0) += 1;

                        // Count by language (inferred from file extension)
                        if let Ok(file_path) = reader.get_symbol_file_path(&symbol) {
                            unique_files.insert(file_path.clone());
                            let path = Path::new(&file_path);
                            let lang = detect_language_from_extension(path);
                            *symbols_by_language.entry(lang.to_string()).or_insert(0) += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read symbols database: {}", e);
                }
            }
        }

        overview_data.insert("total_symbols".to_string(), json!(total_symbols));
        overview_data.insert("code_files".to_string(), json!(unique_files.len()));
        overview_data.insert("symbols_by_type".to_string(), json!(symbols_by_type));
        overview_data.insert(
            "symbols_by_language".to_string(),
            json!(symbols_by_language),
        );

        // 3. Relationship and dependency analysis
        let (total_relationships, connected_symbols, top_referenced_symbols, entry_points) =
            self.analyze_dependencies(&options).await?;

        overview_data.insert(
            "total_relationships".to_string(),
            json!(total_relationships),
        );
        overview_data.insert("connected_symbols".to_string(), json!(connected_symbols));
        overview_data.insert(
            "top_referenced_symbols".to_string(),
            json!(top_referenced_symbols),
        );
        overview_data.insert("entry_points".to_string(), json!(entry_points));

        // 4. File organization patterns
        let (test_files, source_files, doc_files, test_to_code_ratio) =
            self.analyze_file_organization(&unique_files).await?;

        let mut file_organization = HashMap::new();
        file_organization.insert("test_files", test_files);
        file_organization.insert("source_files", source_files);
        file_organization.insert("documentation_files", doc_files);
        overview_data.insert("file_organization".to_string(), json!(file_organization));
        overview_data.insert(
            "test_to_code_ratio".to_string(),
            json!(format!("{:.2}", test_to_code_ratio)),
        );

        // Format output based on requested format
        let formatted_output = self
            .format_overview_output(&overview_data, &options)
            .await?;

        Ok(OverviewResult {
            overview_data,
            formatted_output,
        })
    }

    /// Analyze dependency relationships and find top referenced symbols and entry points
    async fn analyze_dependencies(
        &self,
        options: &OverviewOptions,
    ) -> Result<(usize, usize, Vec<serde_json::Value>, Vec<String>)> {
        let mut total_relationships = 0;
        let mut connected_symbols = 0;
        let mut top_referenced_symbols = Vec::new();
        let mut entry_points = Vec::new();

        let graph_db_path = self.db_path.join("dependency_graph.bin");
        if graph_db_path.exists() {
            if let Ok(graph_binary) = std::fs::read(&graph_db_path) {
                if let Ok(serializable) =
                    bincode::deserialize::<SerializableDependencyGraph>(&graph_binary)
                {
                    total_relationships = serializable.stats.edge_count;
                    connected_symbols = serializable.stats.node_count;

                    // Build a map from UUID to qualified name
                    let mut id_to_name: HashMap<Uuid, String> = HashMap::new();
                    for node in &serializable.nodes {
                        id_to_name.insert(node.symbol_id, node.qualified_name.clone());
                    }

                    // Find top referenced symbols (most incoming edges)
                    let mut reference_counts: HashMap<String, usize> = HashMap::new();
                    for edge in &serializable.edges {
                        if let Some(target_name) = id_to_name.get(&edge.to_id) {
                            *reference_counts.entry(target_name.clone()).or_insert(0) += 1;
                        }
                    }

                    let mut sorted_refs: Vec<_> = reference_counts.into_iter().collect();
                    sorted_refs.sort_by(|a, b| b.1.cmp(&a.1));
                    top_referenced_symbols = sorted_refs
                        .into_iter()
                        .take(options.top_symbols_limit)
                        .map(|(name, count)| json!({"symbol": name, "references": count}))
                        .collect();

                    // Find entry points (symbols with no incoming edges)
                    let mut has_incoming: HashSet<Uuid> = HashSet::new();
                    for edge in &serializable.edges {
                        has_incoming.insert(edge.to_id);
                    }

                    let mut all_symbol_ids: HashSet<Uuid> = HashSet::new();
                    for node in &serializable.nodes {
                        all_symbol_ids.insert(node.symbol_id);
                    }

                    // Find entry points with improved heuristics
                    let mut potential_entry_points: Vec<String> = Vec::new();
                    for symbol_id in all_symbol_ids.difference(&has_incoming) {
                        if let Some(symbol_name) = id_to_name.get(symbol_id) {
                            // Get symbol type if available from nodes
                            let symbol_type = serializable
                                .nodes
                                .iter()
                                .find(|n| n.symbol_id == *symbol_id)
                                .map(|n| format!("{}", n.symbol_type));

                            if is_potential_entry_point(symbol_name, symbol_type.as_deref()) {
                                potential_entry_points.push(symbol_name.clone());
                            }
                        }
                    }

                    // Sort and limit entry points
                    potential_entry_points.sort();
                    entry_points = potential_entry_points
                        .into_iter()
                        .take(options.entry_points_limit)
                        .collect();
                }
            }
        }

        Ok((
            total_relationships,
            connected_symbols,
            top_referenced_symbols,
            entry_points,
        ))
    }

    /// Analyze file organization patterns to determine test files, source files, and documentation
    async fn analyze_file_organization(
        &self,
        unique_files: &HashSet<String>,
    ) -> Result<(usize, usize, usize, f64)> {
        let mut test_files = 0;
        let mut source_files = 0;
        let mut doc_files = 0;

        // Only count files that have actual code symbols extracted
        for file_path in unique_files {
            let path = Path::new(file_path);

            if is_test_file(path) {
                test_files += 1;
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext, "md" | "rst" | "txt" | "adoc" | "org"))
                .unwrap_or(false)
            {
                doc_files += 1;
            } else {
                // This is an actual source code file with symbols
                source_files += 1;
            }
        }

        let test_to_code_ratio = if source_files > 0 {
            test_files as f64 / source_files as f64
        } else {
            0.0
        };

        Ok((test_files, source_files, doc_files, test_to_code_ratio))
    }

    /// Format overview output in human-readable or JSON format
    async fn format_overview_output(
        &self,
        overview_data: &HashMap<String, serde_json::Value>,
        options: &OverviewOptions,
    ) -> Result<String> {
        match options.format.as_str() {
            "json" => {
                let json_output = json!(overview_data);
                Ok(serde_json::to_string_pretty(&json_output)?)
            }
            _ => {
                // Human-readable format
                let mut output = String::new();

                output.push_str("=== CODEBASE OVERVIEW ===\n\n");

                // Scale metrics
                output.push_str("Scale Metrics:\n");
                if let Some(total_files) = overview_data.get("total_files") {
                    output.push_str(&format!("- Total files: {}\n", total_files));
                }
                if let Some(code_files) = overview_data.get("code_files") {
                    output.push_str(&format!("- Code files: {}\n", code_files));
                }
                if let Some(file_org) = overview_data
                    .get("file_organization")
                    .and_then(|v| v.as_object())
                {
                    if let Some(test_files) = file_org.get("test_files") {
                        output.push_str(&format!("- Test files: {}\n", test_files));
                    }
                }
                if let Some(total_symbols) = overview_data.get("total_symbols") {
                    output.push_str(&format!("- Total symbols: {}\n", total_symbols));
                }

                // Symbol types
                if let Some(symbols_by_type) = overview_data
                    .get("symbols_by_type")
                    .and_then(|v| v.as_object())
                {
                    if !symbols_by_type.is_empty() {
                        output.push_str("\nSymbol Types:\n");
                        let mut sorted_types: Vec<_> = symbols_by_type.iter().collect();
                        sorted_types.sort_by(|a, b| {
                            b.1.as_u64().unwrap_or(0).cmp(&a.1.as_u64().unwrap_or(0))
                        });
                        for (sym_type, count) in sorted_types.iter().take(5) {
                            output.push_str(&format!("- {}: {}\n", sym_type, count));
                        }
                    }
                }

                // Languages
                if let Some(symbols_by_lang) = overview_data
                    .get("symbols_by_language")
                    .and_then(|v| v.as_object())
                {
                    if !symbols_by_lang.is_empty() {
                        output.push_str("\nLanguages Detected:\n");
                        let mut sorted_langs: Vec<_> = symbols_by_lang.iter().collect();
                        sorted_langs.sort_by(|a, b| {
                            b.1.as_u64().unwrap_or(0).cmp(&a.1.as_u64().unwrap_or(0))
                        });
                        for (lang, count) in sorted_langs {
                            output.push_str(&format!("- {}: {} symbols\n", lang, count));
                        }
                    }
                }

                // Relationships
                if let Some(total_rel) = overview_data
                    .get("total_relationships")
                    .and_then(|v| v.as_u64())
                {
                    if total_rel > 0 {
                        output.push_str("\nRelationships:\n");
                        output.push_str(&format!("- Total relationships tracked: {}\n", total_rel));
                        if let Some(connected) = overview_data.get("connected_symbols") {
                            output.push_str(&format!("- Connected symbols: {}\n", connected));
                        }
                    }
                }

                // Top referenced symbols
                if let Some(top_refs) = overview_data
                    .get("top_referenced_symbols")
                    .and_then(|v| v.as_array())
                {
                    if !top_refs.is_empty() {
                        output.push_str("\nTop Referenced Symbols:\n");
                        for ref_obj in top_refs {
                            if let Some(obj) = ref_obj.as_object() {
                                if let (Some(symbol), Some(refs)) =
                                    (obj.get("symbol"), obj.get("references"))
                                {
                                    output.push_str(&format!(
                                        "- {} ({} references)\n",
                                        symbol.as_str().unwrap_or(""),
                                        refs
                                    ));
                                }
                            }
                        }
                    }
                }

                // Entry points
                if let Some(entry_points) =
                    overview_data.get("entry_points").and_then(|v| v.as_array())
                {
                    if !entry_points.is_empty() {
                        output.push_str("\nEntry Points (0 callers):\n");
                        for entry in entry_points {
                            if let Some(entry_str) = entry.as_str() {
                                output.push_str(&format!("- {}\n", entry_str));
                            }
                        }
                    }
                }

                // File organization
                if let Some(file_org) = overview_data
                    .get("file_organization")
                    .and_then(|v| v.as_object())
                {
                    output.push_str("\nFile Organization:\n");
                    if let Some(source_files) = file_org.get("source_files") {
                        output.push_str(&format!("- Source code: {} files\n", source_files));
                    }
                    if let Some(test_files) = file_org.get("test_files") {
                        output.push_str(&format!("- Test files: {} files\n", test_files));
                    }
                    if let Some(doc_files) = file_org.get("documentation_files") {
                        output.push_str(&format!("- Documentation: {} files\n", doc_files));
                    }
                }

                // Test coverage
                output.push_str("\nTest Coverage Indicators:\n");
                if let Some(test_ratio) = overview_data
                    .get("test_to_code_ratio")
                    .and_then(|v| v.as_str())
                {
                    output.push_str(&format!("- Test-to-code ratio: {}\n", test_ratio));
                }

                // Calculate estimated test coverage based on test-to-code ratio
                if let Some(file_org) = overview_data
                    .get("file_organization")
                    .and_then(|v| v.as_object())
                {
                    if let (Some(source_files), Some(test_files)) = (
                        file_org.get("source_files").and_then(|v| v.as_u64()),
                        file_org.get("test_files").and_then(|v| v.as_u64()),
                    ) {
                        if source_files > 0 {
                            let test_to_code_ratio = test_files as f64 / source_files as f64;
                            let coverage_estimate = 90.0 * (test_to_code_ratio * 0.8).tanh();
                            let final_estimate = if test_files > 0 {
                                (coverage_estimate + 10.0).min(90.0)
                            } else {
                                0.0
                            };
                            output.push_str(&format!(
                                "- Estimated test coverage: {:.0}%\n",
                                final_estimate
                            ));
                        }
                    }
                }

                Ok(output)
            }
        }
    }
}
