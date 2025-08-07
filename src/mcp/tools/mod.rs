pub mod analytics_tools;
/// MCP Tools Implementation
///
/// This module contains the actual tool implementations that expose
/// KotaDB functionality through the Model Context Protocol.
pub mod document_tools;
pub mod graph_tools;
// pub mod search_tools; // Temporarily disabled due to compilation issues

use crate::mcp::types::*;
use anyhow::Result;
use std::sync::Arc;

/// Trait for MCP tool handlers
#[async_trait::async_trait]
pub trait MCPToolHandler {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value>;
    fn get_tool_definitions(&self) -> Vec<ToolDefinition>;
}

/// Main tool registry that coordinates all MCP tools
pub struct MCPToolRegistry {
    pub document_tools: Option<Arc<document_tools::DocumentTools>>,
    // pub search_tools: Option<Arc<search_tools::SearchTools>>, // Disabled
    // pub analytics_tools: Option<Arc<analytics_tools::AnalyticsTools>>,
    // pub graph_tools: Option<Arc<graph_tools::GraphTools>>,
}

impl Default for MCPToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MCPToolRegistry {
    pub fn new() -> Self {
        Self {
            document_tools: None,
            // search_tools: None, // Disabled
            // analytics_tools: None,
            // graph_tools: None,
        }
    }

    /// Register document tools
    pub fn with_document_tools(mut self, tools: Arc<document_tools::DocumentTools>) -> Self {
        self.document_tools = Some(tools);
        self
    }

    // /// Register search tools (disabled)
    // pub fn with_search_tools(mut self, tools: Arc<search_tools::SearchTools>) -> Self {
    //     self.search_tools = Some(tools);
    //     self
    // }

    // /// Register analytics tools
    // pub fn with_analytics_tools(mut self, tools: Arc<analytics_tools::AnalyticsTools>) -> Self {
    //     self.analytics_tools = Some(tools);
    //     self
    // }

    // /// Register graph tools
    // pub fn with_graph_tools(mut self, tools: Arc<graph_tools::GraphTools>) -> Self {
    //     self.graph_tools = Some(tools);
    //     self
    // }

    /// Get all available tool definitions
    pub fn get_all_tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut definitions = Vec::new();

        if let Some(tools) = &self.document_tools {
            definitions.extend(tools.get_tool_definitions());
        }
        // if let Some(tools) = &self.search_tools {
        //     definitions.extend(tools.get_tool_definitions());
        // }
        // if let Some(tools) = &self.analytics_tools {
        //     definitions.extend(tools.get_tool_definitions());
        // }
        // if let Some(tools) = &self.graph_tools {
        //     definitions.extend(tools.get_tool_definitions());
        // }

        definitions
    }

    /// Handle a tool call by routing to the appropriate handler
    pub async fn handle_tool_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        tracing::debug!("Handling tool call: {}", method);

        // Route to appropriate tool handler based on method prefix
        match method {
            m if m.starts_with("kotadb://document_") => {
                if let Some(tools) = &self.document_tools {
                    tools.handle_call(method, params).await
                } else {
                    Err(anyhow::anyhow!("Document tools not enabled"))
                }
            }
            // Search tools disabled
            // m if m.starts_with("kotadb://text_search")
            //     || m.starts_with("kotadb://semantic_search") =>
            // {
            //     if let Some(tools) = &self.search_tools {
            //         tools.handle_call(method, params).await
            //     } else {
            //         Err(anyhow::anyhow!("Search tools not enabled"))
            //     }
            // }
            // m if m.starts_with("kotadb://analytics_") || m.starts_with("kotadb://health_check") => {
            //     if let Some(tools) = &self.analytics_tools {
            //         tools.handle_call(method, params).await
            //     } else {
            //         Err(anyhow::anyhow!("Analytics tools not enabled"))
            //     }
            // }
            // m if m.starts_with("kotadb://graph_") => {
            //     if let Some(tools) = &self.graph_tools {
            //         tools.handle_call(method, params).await
            //     } else {
            //         Err(anyhow::anyhow!("Graph tools not enabled"))
            //     }
            // }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
