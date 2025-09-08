//! Git repository integration for KotaDB
//!
//! This module provides functionality for ingesting git repositories into KotaDB,
//! enabling codebase analysis and intelligence features.

mod document_metadata;
mod file_organization;
mod ingestion;
mod repository;
pub mod types;

pub use document_metadata::{GitDocument, GitMetadata, RepositoryOrganizationConfig};
pub use file_organization::{FileOrganizationManager, FileOrganizationStats};
pub use ingestion::{IngestResult, IngestionConfig, ProgressCallback, RepositoryIngester};
pub use repository::GitRepository;
pub use types::{CommitInfo, FileEntry, IngestionOptions, RepositoryMetadata};

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_git_module_imports() -> Result<()> {
        // Basic test to ensure module structure is correct
        let _temp = TempDir::new()?;
        Ok(())
    }
}
