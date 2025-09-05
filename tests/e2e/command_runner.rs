// CommandRunner: E2E CLI Command Execution Helper
// Executes KotaDB CLI commands with proper error handling and validation
// Following Stage 6: Component Library patterns with comprehensive observability

use crate::e2e::{E2ETestResult, TestEnvironment};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;

/// Helper for executing KotaDB CLI commands in E2E tests
///
/// Features:
/// - Automatic timeout handling
/// - Comprehensive output capture
/// - Performance timing
/// - Error context preservation
/// - Command sequence validation
pub struct CommandRunner {
    /// Path to the KotaDB binary
    binary_path: PathBuf,
    /// Default timeout for commands
    default_timeout: Duration,
}

impl CommandRunner {
    /// Creates a new CommandRunner for the given test environment
    pub fn new(env: &TestEnvironment) -> Self {
        Self {
            binary_path: env.kotadb_binary_path(),
            default_timeout: Duration::from_secs(30), // 30 second default timeout
        }
    }

    /// Creates a CommandRunner with custom timeout
    pub fn with_timeout(env: &TestEnvironment, timeout: Duration) -> Self {
        Self {
            binary_path: env.kotadb_binary_path(),
            default_timeout: timeout,
        }
    }

    /// Execute a KotaDB command with arguments
    pub async fn run(&self, args: &[&str]) -> Result<E2ETestResult> {
        self.run_with_timeout(args, self.default_timeout).await
    }

    /// Execute a KotaDB command with specific timeout
    pub async fn run_with_timeout(
        &self,
        args: &[&str],
        timeout: Duration,
    ) -> Result<E2ETestResult> {
        let start_time = Instant::now();

        let mut command = Command::new(&self.binary_path);
        command.args(args);

        // Add timeout
        let output = tokio::time::timeout(timeout, command.output())
            .await
            .context(format!(
                "Command timed out after {:?}: kotadb {}",
                timeout,
                args.join(" ")
            ))?
            .context("Failed to execute KotaDB command")?;

        let duration = start_time.elapsed();
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(E2ETestResult::new(
            output.status.success(),
            output_str,
            stderr_str,
            duration.as_millis(),
        ))
    }

    /// Execute an index-codebase command
    pub async fn index_codebase(
        &self,
        db_path: &Path,
        codebase_path: &Path,
        with_symbols: bool,
    ) -> Result<E2ETestResult> {
        let mut args = vec![
            "-d",
            db_path.to_str().context("Invalid database path")?,
            "index-codebase",
            codebase_path.to_str().context("Invalid codebase path")?,
        ];

        if !with_symbols {
            args.push("--no-symbols");
        }
        // with_symbols=true is the default, no flag needed

        // Indexing can take longer, use extended timeout
        self.run_with_timeout(&args, Duration::from_secs(120)).await
    }

    /// Execute a stats command
    pub async fn stats(&self, db_path: &Path, with_symbols: bool) -> Result<E2ETestResult> {
        let mut args = vec![
            "-d",
            db_path.to_str().context("Invalid database path")?,
            "stats",
        ];

        if with_symbols {
            args.push("--symbols");
        }

        self.run(&args).await
    }

    /// Execute a search-code command
    pub async fn search_code(&self, db_path: &Path, query: &str) -> Result<E2ETestResult> {
        let args = vec![
            "-d",
            db_path.to_str().context("Invalid database path")?,
            "search-code",
            query,
        ];

        self.run(&args).await
    }

    /// Execute a search-symbols command
    pub async fn search_symbols(&self, db_path: &Path, pattern: &str) -> Result<E2ETestResult> {
        let args = vec![
            "-d",
            db_path.to_str().context("Invalid database path")?,
            "search-symbols",
            pattern,
        ];

        self.run(&args).await
    }

    /// Execute a find-callers command
    pub async fn find_callers(&self, db_path: &Path, symbol: &str) -> Result<E2ETestResult> {
        let args = vec![
            "-d",
            db_path.to_str().context("Invalid database path")?,
            "find-callers",
            symbol,
        ];

        self.run(&args).await
    }

    /// Execute an analyze-impact command
    pub async fn analyze_impact(&self, db_path: &Path, symbol: &str) -> Result<E2ETestResult> {
        let args = vec![
            "-d",
            db_path.to_str().context("Invalid database path")?,
            "analyze-impact",
            symbol,
        ];

        self.run(&args).await
    }

    /// Validate that a command result meets performance expectations
    pub fn validate_performance(
        &self,
        result: &E2ETestResult,
        max_duration_ms: u128,
    ) -> Result<()> {
        if result.duration_ms > max_duration_ms {
            anyhow::bail!(
                "Command took {}ms, expected <{}ms. Performance regression detected.",
                result.duration_ms,
                max_duration_ms
            );
        }
        Ok(())
    }

    /// Validate that a command succeeded
    pub fn validate_success(&self, result: &E2ETestResult) -> Result<()> {
        if !result.success {
            anyhow::bail!(
                "Command failed. STDERR: {}\nSTDOUT: {}",
                result.stderr,
                result.output
            );
        }
        Ok(())
    }

    /// Validate that command output contains expected content
    pub fn validate_output_contains(&self, result: &E2ETestResult, expected: &str) -> Result<()> {
        if !result.output.contains(expected) {
            anyhow::bail!(
                "Command output does not contain '{}'. Output: {}",
                expected,
                result.output
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e2e::TestEnvironment;

    #[tokio::test]
    async fn test_command_runner_creation() -> Result<()> {
        let env = TestEnvironment::new()?;
        let runner = CommandRunner::new(&env);

        // Verify binary path is set correctly
        assert!(runner.binary_path.ends_with("kotadb"));

        Ok(())
    }

    #[tokio::test]
    async fn test_command_timeout() -> Result<()> {
        let env = TestEnvironment::new()?;
        let runner = CommandRunner::with_timeout(&env, Duration::from_millis(1));

        // This should timeout (1ms is way too short for any real command)
        let result = runner.run(&["--help"]).await;

        // Should return error due to timeout
        assert!(result.is_err());

        Ok(())
    }
}
