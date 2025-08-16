//! Multi-language code parsing for KotaDB using tree-sitter
//!
//! This module provides sophisticated code parsing capabilities to extract
//! structural information from source code files, enabling advanced codebase
//! analysis features like symbol extraction, dependency mapping, and
//! intelligent code queries.

#[cfg(feature = "tree-sitter-parsing")]
mod tree_sitter;

#[cfg(feature = "tree-sitter-parsing")]
pub use tree_sitter::{
    CodeParser, ParsedCode, ParsedSymbol, ParsingConfig, SupportedLanguage, SymbolKind, SymbolType,
};

#[cfg(not(feature = "tree-sitter-parsing"))]
pub mod stub {
    //! Stub implementations when tree-sitter parsing is not enabled
    use anyhow::{anyhow, Result};

    pub struct CodeParser;

    #[derive(Debug, Clone)]
    pub struct ParsedCode {
        pub language: SupportedLanguage,
        pub stats: ParsedStats,
        pub symbols: Vec<ParsedSymbol>,
        pub errors: Vec<ParseError>,
    }

    #[derive(Debug, Clone)]
    pub struct ParsedStats {
        pub total_nodes: usize,
        pub named_nodes: usize,
        pub max_depth: usize,
        pub error_count: usize,
    }

    #[derive(Debug, Clone)]
    pub struct ParsedSymbol {
        pub name: String,
        pub kind: String,
        pub line: usize,
        pub column: usize,
    }

    #[derive(Debug, Clone)]
    pub struct ParseError {
        pub message: String,
        pub line: usize,
        pub column: usize,
    }

    #[derive(Debug, Clone)]
    pub enum SupportedLanguage {
        Rust,
    }

    impl CodeParser {
        pub fn new() -> Result<Self> {
            Err(anyhow!(
                "Tree-sitter parsing not enabled. Enable the 'tree-sitter-parsing' feature."
            ))
        }

        pub fn parse_content(
            &self,
            _content: &str,
            _language: SupportedLanguage,
        ) -> Result<ParsedCode> {
            Err(anyhow!(
                "Tree-sitter parsing not enabled. Enable the 'tree-sitter-parsing' feature."
            ))
        }
    }
}

#[cfg(not(feature = "tree-sitter-parsing"))]
pub use stub::*;

#[cfg(test)]
mod tests {
    use anyhow::Result;

    #[tokio::test]
    async fn test_parsing_module_imports() -> Result<()> {
        // Basic test to ensure module structure is correct
        #[cfg(feature = "tree-sitter-parsing")]
        {
            use crate::parsing::SupportedLanguage;
            let _rust_lang = SupportedLanguage::Rust;
        }
        Ok(())
    }
}
