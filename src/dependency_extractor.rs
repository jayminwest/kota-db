//! Dependency extraction and call graph building for code analysis
//!
//! This module extends the symbol extraction pipeline to capture relationships
//! between symbols including function calls, type usage, imports, and module dependencies.

use anyhow::{Context, Result};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use tracing::{instrument, warn};
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};
use uuid::Uuid;

use crate::parsing::{CodeParser, ParsedCode, ParsedSymbol, SupportedLanguage, SymbolType};
use crate::symbol_storage::{RelationType, SymbolEntry};

/// Dependency graph representation for code analysis
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// The underlying directed graph
    pub graph: DiGraph<SymbolNode, DependencyEdge>,
    /// Mapping from symbol ID to graph node index
    pub symbol_to_node: HashMap<Uuid, NodeIndex>,
    /// Mapping from qualified name to symbol ID for resolution
    pub name_to_symbol: HashMap<String, Uuid>,
    /// Import mappings for each file
    pub file_imports: HashMap<PathBuf, Vec<ImportStatement>>,
    /// Statistics about the graph
    pub stats: GraphStats,
}

/// Serializable representation of the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableDependencyGraph {
    /// All nodes in the graph
    pub nodes: Vec<SymbolNode>,
    /// All edges in the graph with source and target IDs
    pub edges: Vec<SerializableEdge>,
    /// Mapping from qualified name to symbol ID
    pub name_to_symbol: HashMap<String, Uuid>,
    /// Import mappings for each file
    pub file_imports: HashMap<PathBuf, Vec<ImportStatement>>,
    /// Statistics about the graph
    pub stats: GraphStats,
}

/// Serializable edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEdge {
    /// Source symbol ID
    pub from_id: Uuid,
    /// Target symbol ID
    pub to_id: Uuid,
    /// Edge data
    pub edge: DependencyEdge,
}

/// Node in the dependency graph representing a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    /// Symbol ID from the symbol storage
    pub symbol_id: Uuid,
    /// Fully qualified name of the symbol
    pub qualified_name: String,
    /// Type of the symbol
    pub symbol_type: SymbolType,
    /// File path containing this symbol
    pub file_path: PathBuf,
    /// Number of incoming dependencies
    pub in_degree: usize,
    /// Number of outgoing dependencies
    pub out_degree: usize,
}

/// Edge in the dependency graph representing a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Type of relationship
    pub relation_type: RelationType,
    /// Line number where the reference occurs
    pub line_number: usize,
    /// Column number where the reference occurs
    pub column_number: usize,
    /// Context snippet around the reference
    pub context: Option<String>,
}

/// Import statement representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement {
    /// The import path (e.g., "std::collections::HashMap")
    pub path: String,
    /// Imported items (e.g., ["HashMap", "HashSet"])
    pub items: Vec<String>,
    /// Alias if any (e.g., "use foo as bar")
    pub alias: Option<String>,
    /// Line number of the import
    pub line_number: usize,
    /// Whether it's a wildcard import (use foo::*)
    pub is_wildcard: bool,
}

/// Statistics about the dependency graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of nodes (symbols)
    pub node_count: usize,
    /// Total number of edges (dependencies)
    pub edge_count: usize,
    /// Number of files analyzed
    pub file_count: usize,
    /// Number of import statements
    pub import_count: usize,
    /// Strongly connected components (potential circular dependencies)
    pub scc_count: usize,
    /// Maximum dependency depth
    pub max_depth: usize,
    /// Average dependencies per symbol
    pub avg_dependencies: f64,
}

/// Reference found in code (function call, type usage, etc.)
#[derive(Debug, Clone)]
pub struct CodeReference {
    /// Name being referenced
    pub name: String,
    /// Type of reference
    pub ref_type: ReferenceType,
    /// Location in source
    pub line: usize,
    pub column: usize,
    /// Full text of the reference
    pub text: String,
}

/// Type of reference found in code
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceType {
    FunctionCall,
    TypeUsage,
    TraitImpl,
    MacroInvocation,
    FieldAccess,
    MethodCall,
}

/// Dependency extractor that analyzes code for relationships
pub struct DependencyExtractor {
    /// Code parser for symbol extraction (kept for future use)
    #[allow(dead_code)]
    parser: CodeParser,
    /// Tree-sitter queries for different languages
    queries: HashMap<SupportedLanguage, DependencyQueries>,
}

/// Tree-sitter queries for extracting dependencies
struct DependencyQueries {
    /// Query for function calls
    function_calls: Query,
    /// Query for type references
    type_references: Query,
    /// Query for imports
    imports: Query,
    /// Query for method calls
    method_calls: Query,
}

impl DependencyExtractor {
    /// Create a new dependency extractor
    pub fn new() -> Result<Self> {
        let parser = CodeParser::new()?;
        let mut queries = HashMap::new();

        // Initialize Rust queries
        let rust_queries = Self::init_rust_queries()?;
        queries.insert(SupportedLanguage::Rust, rust_queries);

        Ok(Self { parser, queries })
    }

    /// Initialize tree-sitter queries for Rust
    fn init_rust_queries() -> Result<DependencyQueries> {
        let language = tree_sitter_rust::LANGUAGE.into();

        // Query for function calls
        let function_calls = Query::new(
            &language,
            r#"
            (call_expression
                function: (identifier) @function_name)
            (call_expression
                function: (scoped_identifier
                    name: (identifier) @function_name))
            (call_expression
                function: (field_expression
                    field: (field_identifier) @method_name))
            "#,
        )
        .context("Failed to create function calls query")?;

        // Query for type references
        let type_references = Query::new(
            &language,
            r#"
            (type_identifier) @type_name
            (scoped_type_identifier
                name: (type_identifier) @type_name)
            (generic_type
                type: (type_identifier) @type_name)
            "#,
        )
        .context("Failed to create type references query")?;

        // Query for imports
        let imports = Query::new(
            &language,
            r#"
            (use_declaration
                argument: (scoped_identifier) @import_path)
            (use_declaration
                argument: (use_list) @import_list)
            (use_declaration
                argument: (use_as_clause
                    path: (scoped_identifier) @import_path
                    alias: (identifier) @import_alias))
            "#,
        )
        .context("Failed to create imports query")?;

        // Query for method calls (using Rust's actual node type)
        let method_calls = Query::new(
            &language,
            r#"
            (call_expression
                function: (field_expression
                    field: (field_identifier) @method_name))
            "#,
        )
        .context("Failed to create method calls query")?;

        Ok(DependencyQueries {
            function_calls,
            type_references,
            imports,
            method_calls,
        })
    }

    /// Extract dependencies from a parsed code file
    #[instrument(skip(self, parsed_code, content))]
    pub fn extract_dependencies(
        &self,
        parsed_code: &ParsedCode,
        content: &str,
        file_path: &Path,
    ) -> Result<DependencyAnalysis> {
        let mut analysis = DependencyAnalysis {
            file_path: file_path.to_path_buf(),
            imports: Vec::new(),
            references: Vec::new(),
            symbols: parsed_code.symbols.clone(),
        };

        // Parse the content again to get the tree for queries
        let mut parser = Parser::new();
        let language = parsed_code.language.tree_sitter_language()?;
        parser.set_language(&language)?;

        let tree = parser
            .parse(content, None)
            .context("Failed to parse content for dependency extraction")?;

        // Extract imports
        analysis.imports = self.extract_imports(&tree, content, parsed_code.language)?;

        // Extract references (function calls, type usage, etc.)
        analysis.references = self.extract_references(&tree, content, parsed_code.language)?;

        Ok(analysis)
    }

    /// Extract import statements from the parse tree
    fn extract_imports(
        &self,
        tree: &Tree,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<Vec<ImportStatement>> {
        let queries = self
            .queries
            .get(&language)
            .context("No queries for language")?;

        let mut imports = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&queries.imports, tree.root_node(), content.as_bytes());

        while let Some(match_) = matches.next() {
            let mut import = ImportStatement {
                path: String::new(),
                items: Vec::new(),
                alias: None,
                line_number: 0,
                is_wildcard: false,
            };

            for capture in match_.captures {
                let node = capture.node;
                let text = node.utf8_text(content.as_bytes())?;

                match queries.imports.capture_names()[capture.index as usize] {
                    "import_path" => {
                        import.path = text.to_string();
                        import.line_number = node.start_position().row + 1;
                    }
                    "import_alias" => {
                        import.alias = Some(text.to_string());
                    }
                    "import_list" => {
                        // Parse the use list to extract individual items
                        import.items = self.parse_use_list(node, content)?;
                    }
                    _ => {}
                }
            }

            // Check for wildcard imports
            if import.path.ends_with("*") {
                import.is_wildcard = true;
            }

            if !import.path.is_empty() {
                imports.push(import);
            }
        }

        Ok(imports)
    }

    /// Parse a use list node to extract individual imported items
    fn parse_use_list(&self, node: Node, content: &str) -> Result<Vec<String>> {
        let mut items = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(content.as_bytes()) {
                    items.push(text.to_string());
                }
            }
        }

        Ok(items)
    }

    /// Extract references (function calls, type usage, etc.) from the parse tree
    fn extract_references(
        &self,
        tree: &Tree,
        content: &str,
        language: SupportedLanguage,
    ) -> Result<Vec<CodeReference>> {
        let queries = self
            .queries
            .get(&language)
            .context("No queries for language")?;

        let mut references = Vec::new();

        // Extract function calls
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(
            &queries.function_calls,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let text = node.utf8_text(content.as_bytes())?;
                let pos = node.start_position();

                references.push(CodeReference {
                    name: text.to_string(),
                    ref_type: ReferenceType::FunctionCall,
                    line: pos.row + 1,
                    column: pos.column,
                    text: text.to_string(),
                });
            }
        }

        // Extract type references
        let mut matches = cursor.matches(
            &queries.type_references,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let text = node.utf8_text(content.as_bytes())?;
                let pos = node.start_position();

                references.push(CodeReference {
                    name: text.to_string(),
                    ref_type: ReferenceType::TypeUsage,
                    line: pos.row + 1,
                    column: pos.column,
                    text: text.to_string(),
                });
            }
        }

        // Extract method calls
        let mut matches =
            cursor.matches(&queries.method_calls, tree.root_node(), content.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let text = node.utf8_text(content.as_bytes())?;
                let pos = node.start_position();

                references.push(CodeReference {
                    name: text.to_string(),
                    ref_type: ReferenceType::MethodCall,
                    line: pos.row + 1,
                    column: pos.column,
                    text: text.to_string(),
                });
            }
        }

        Ok(references)
    }

    /// Build a complete dependency graph from multiple analyzed files
    pub fn build_dependency_graph(
        &self,
        analyses: Vec<DependencyAnalysis>,
        symbol_entries: &[SymbolEntry],
    ) -> Result<DependencyGraph> {
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();
        let mut name_to_symbol = HashMap::new();
        let mut file_imports = HashMap::new();

        // First pass: Create nodes for all symbols
        for entry in symbol_entries {
            let node = SymbolNode {
                symbol_id: entry.id,
                qualified_name: entry.qualified_name.clone(),
                symbol_type: entry.symbol.symbol_type.clone(),
                file_path: entry.file_path.clone(),
                in_degree: 0,
                out_degree: 0,
            };

            let node_idx = graph.add_node(node);
            symbol_to_node.insert(entry.id, node_idx);
            name_to_symbol.insert(entry.qualified_name.clone(), entry.id);

            // Also index by simple name for fallback resolution
            name_to_symbol.insert(entry.symbol.name.clone(), entry.id);
        }

        // Second pass: Create edges based on references
        for analysis in &analyses {
            file_imports.insert(analysis.file_path.clone(), analysis.imports.clone());

            // Find symbols defined in this file
            let file_symbols: Vec<_> = symbol_entries
                .iter()
                .filter(|e| e.file_path == analysis.file_path)
                .collect();

            for reference in &analysis.references {
                // Try to resolve the reference to a symbol
                if let Some(target_id) =
                    self.resolve_reference(&reference.name, &analysis.imports, &name_to_symbol)
                {
                    // Find the source symbol (the one containing this reference)
                    if let Some(source_symbol) =
                        self.find_containing_symbol(reference.line, &file_symbols)
                    {
                        if let (Some(&source_idx), Some(&target_idx)) = (
                            symbol_to_node.get(&source_symbol.id),
                            symbol_to_node.get(&target_id),
                        ) {
                            // Don't add self-references
                            if source_idx != target_idx {
                                let edge = DependencyEdge {
                                    relation_type: match reference.ref_type {
                                        ReferenceType::FunctionCall => RelationType::Calls,
                                        ReferenceType::TypeUsage => {
                                            RelationType::Custom("uses_type".to_string())
                                        }
                                        ReferenceType::MethodCall => RelationType::Calls,
                                        _ => RelationType::Custom("references".to_string()),
                                    },
                                    line_number: reference.line,
                                    column_number: reference.column,
                                    context: Some(reference.text.clone()),
                                };

                                graph.add_edge(source_idx, target_idx, edge);
                            }
                        }
                    }
                }
            }
        }

        // Calculate statistics
        let stats = self.calculate_graph_stats(&graph, &analyses);

        // Update in/out degrees for nodes
        for node_idx in graph.node_indices() {
            let in_degree = graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .count();
            let out_degree = graph
                .edges_directed(node_idx, petgraph::Direction::Outgoing)
                .count();

            if let Some(node) = graph.node_weight_mut(node_idx) {
                node.in_degree = in_degree;
                node.out_degree = out_degree;
            }
        }

        Ok(DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol,
            file_imports,
            stats,
        })
    }

    /// Resolve a reference name to a symbol ID
    fn resolve_reference(
        &self,
        name: &str,
        imports: &[ImportStatement],
        name_to_symbol: &HashMap<String, Uuid>,
    ) -> Option<Uuid> {
        // Direct lookup
        if let Some(&id) = name_to_symbol.get(name) {
            return Some(id);
        }

        // Try with import prefixes
        for import in imports {
            // Check if this import could resolve the reference
            if import.items.contains(&name.to_string()) {
                let qualified = format!("{}::{}", import.path, name);
                if let Some(&id) = name_to_symbol.get(&qualified) {
                    return Some(id);
                }
            }

            // Check if it's a path that starts with an imported module
            if name.contains("::") {
                let parts: Vec<&str> = name.split("::").collect();
                if !parts.is_empty() && import.items.contains(&parts[0].to_string()) {
                    let qualified = format!("{}::{}", import.path, name);
                    if let Some(&id) = name_to_symbol.get(&qualified) {
                        return Some(id);
                    }
                }
            }
        }

        None
    }

    /// Find the symbol that contains a given line number
    fn find_containing_symbol<'a>(
        &self,
        line: usize,
        symbols: &[&'a SymbolEntry],
    ) -> Option<&'a SymbolEntry> {
        symbols
            .iter()
            .filter(|s| s.symbol.start_line <= line && s.symbol.end_line >= line)
            .min_by_key(|s| s.symbol.end_line - s.symbol.start_line)
            .copied()
    }

    /// Calculate statistics for the dependency graph
    fn calculate_graph_stats(
        &self,
        graph: &DiGraph<SymbolNode, DependencyEdge>,
        analyses: &[DependencyAnalysis],
    ) -> GraphStats {
        let node_count = graph.node_count();
        let edge_count = graph.edge_count();
        let file_count = analyses.len();
        let import_count: usize = analyses.iter().map(|a| a.imports.len()).sum();

        // Find strongly connected components
        let scc = petgraph::algo::kosaraju_scc(graph);
        let scc_count = scc.iter().filter(|component| component.len() > 1).count();

        // Calculate maximum depth using BFS from root nodes
        let max_depth = self.calculate_max_depth(graph);

        let avg_dependencies = if node_count > 0 {
            edge_count as f64 / node_count as f64
        } else {
            0.0
        };

        GraphStats {
            node_count,
            edge_count,
            file_count,
            import_count,
            scc_count,
            max_depth,
            avg_dependencies,
        }
    }

    /// Calculate the maximum dependency depth in the graph
    fn calculate_max_depth(&self, graph: &DiGraph<SymbolNode, DependencyEdge>) -> usize {
        let mut max_depth = 0;

        // Find root nodes (nodes with no incoming edges)
        let root_nodes: Vec<_> = graph
            .node_indices()
            .filter(|&idx| {
                graph
                    .edges_directed(idx, petgraph::Direction::Incoming)
                    .count()
                    == 0
            })
            .collect();

        // BFS from each root to find maximum depth
        for root in root_nodes {
            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            queue.push_back((root, 0));

            while let Some((node, depth)) = queue.pop_front() {
                if visited.contains(&node) {
                    continue;
                }
                visited.insert(node);
                max_depth = max_depth.max(depth);

                for edge in graph.edges(node) {
                    queue.push_back((edge.target(), depth + 1));
                }
            }
        }

        max_depth
    }
}

/// Result of dependency analysis for a single file
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// Path to the analyzed file
    pub file_path: PathBuf,
    /// Import statements found
    pub imports: Vec<ImportStatement>,
    /// References found in the code
    pub references: Vec<CodeReference>,
    /// Symbols defined in this file
    pub symbols: Vec<ParsedSymbol>,
}

impl DependencyGraph {
    /// Find all dependencies of a given symbol
    pub fn find_dependencies(&self, symbol_id: Uuid) -> Vec<(Uuid, RelationType)> {
        if let Some(&node_idx) = self.symbol_to_node.get(&symbol_id) {
            self.graph
                .edges(node_idx)
                .map(|edge| {
                    let target_node = &self.graph[edge.target()];
                    (target_node.symbol_id, edge.weight().relation_type.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find all symbols that depend on a given symbol
    pub fn find_dependents(&self, symbol_id: Uuid) -> Vec<(Uuid, RelationType)> {
        if let Some(&node_idx) = self.symbol_to_node.get(&symbol_id) {
            self.graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .map(|edge| {
                    let source_node = &self.graph[edge.source()];
                    (source_node.symbol_id, edge.weight().relation_type.clone())
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Detect circular dependencies in the graph
    pub fn find_circular_dependencies(&self) -> Vec<Vec<Uuid>> {
        let scc = petgraph::algo::kosaraju_scc(&self.graph);

        scc.into_iter()
            .filter(|component| component.len() > 1)
            .map(|component| {
                component
                    .into_iter()
                    .map(|idx| self.graph[idx].symbol_id)
                    .collect()
            })
            .collect()
    }

    /// Generate visualization data in DOT format
    pub fn to_dot(&self) -> String {
        use petgraph::dot::{Config, Dot};

        let dot = Dot::with_config(&self.graph, &[Config::EdgeNoLabel]);
        format!("{:?}", dot)
    }

    /// Convert to serializable representation
    pub fn to_serializable(&self) -> SerializableDependencyGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Collect all nodes
        for node_idx in self.graph.node_indices() {
            if let Some(node) = self.graph.node_weight(node_idx) {
                nodes.push(node.clone());
            }
        }

        // Collect all edges
        for edge_ref in self.graph.edge_references() {
            let source_node = &self.graph[edge_ref.source()];
            let target_node = &self.graph[edge_ref.target()];

            edges.push(SerializableEdge {
                from_id: source_node.symbol_id,
                to_id: target_node.symbol_id,
                edge: edge_ref.weight().clone(),
            });
        }

        SerializableDependencyGraph {
            nodes,
            edges,
            name_to_symbol: self.name_to_symbol.clone(),
            file_imports: self.file_imports.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Reconstruct from serializable representation
    pub fn from_serializable(serializable: SerializableDependencyGraph) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut symbol_to_node = HashMap::new();

        // Add all nodes
        for node in serializable.nodes {
            let node_idx = graph.add_node(node.clone());
            symbol_to_node.insert(node.symbol_id, node_idx);
        }

        // Add all edges
        for edge_data in serializable.edges {
            if let (Some(&from_idx), Some(&to_idx)) = (
                symbol_to_node.get(&edge_data.from_id),
                symbol_to_node.get(&edge_data.to_id),
            ) {
                graph.add_edge(from_idx, to_idx, edge_data.edge);
            }
        }

        Ok(DependencyGraph {
            graph,
            symbol_to_node,
            name_to_symbol: serializable.name_to_symbol,
            file_imports: serializable.file_imports,
            stats: serializable.stats,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dependency_extraction() {
        let extractor = DependencyExtractor::new().unwrap();

        let rust_code = r#"
use std::collections::HashMap;
use crate::utils::helper;

fn process_data(data: HashMap<String, i32>) -> i32 {
    let result = helper::calculate(data);
    validate_result(result)
}

fn validate_result(value: i32) -> i32 {
    if value > 0 {
        value * 2
    } else {
        0
    }
}
"#;

        // Parse the code first
        let mut parser = CodeParser::new().unwrap();
        let parsed = parser
            .parse_content(rust_code, SupportedLanguage::Rust)
            .unwrap();

        // Extract dependencies
        let path = PathBuf::from("test.rs");
        let analysis = extractor
            .extract_dependencies(&parsed, rust_code, &path)
            .unwrap();

        // Check imports
        assert_eq!(analysis.imports.len(), 2);
        assert!(analysis.imports.iter().any(|i| i.path.contains("HashMap")));
        assert!(analysis.imports.iter().any(|i| i.path.contains("helper")));

        // Check references
        assert!(analysis.references.iter().any(|r| r.name == "HashMap"));
        assert!(analysis.references.iter().any(|r| r.name == "calculate"));
        assert!(analysis
            .references
            .iter()
            .any(|r| r.name == "validate_result"));
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        // This test would require setting up a graph with circular dependencies
        // and verifying they're detected correctly
    }
}
