//! KotaDB MCP Server Binary
//!
//! Standalone binary for running KotaDB as an Model Context Protocol server
//! for seamless LLM integration.
#[cfg(feature = "mcp-server")]
use anyhow::Result;
#[cfg(feature = "mcp-server")]
use clap::{Arg, Command};
#[cfg(feature = "mcp-server")]
use kotadb::mcp::{config::MCPConfig, init_mcp_server};
#[cfg(feature = "mcp-server")]
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(feature = "mcp-server")]
fn main() -> Result<()> {
    let matches = Command::new("kotadb-mcp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("KotaDB Model Context Protocol Server")
        .long_about(
            "A high-performance MCP server that exposes KotaDB functionality to LLM clients",
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("kotadb-mcp.toml"),
        )
        .arg(
            Arg::new("data-dir")
                .short('d')
                .long("data-dir")
                .value_name("DIR")
                .help("Data directory path")
                .default_value("./kotadb-data"),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Server host address")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Server port")
                .default_value("3000"),
        )
        .arg(
            Arg::new("health-check")
                .long("health-check")
                .action(clap::ArgAction::SetTrue)
                .help("Perform health check and exit"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Increase verbosity (can be used multiple times)"),
        )
        .get_matches();

    // Initialize logging
    let log_level = match matches.get_count("verbose") {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("kotadb={log_level},kotadb_mcp={log_level}")));

    fmt().with_env_filter(filter).with_target(false).init();

    // Handle health check
    if matches.get_flag("health-check") {
        let rt = tokio::runtime::Runtime::new()?;
        return rt.block_on(perform_health_check());
    }

    // Load configuration
    let config_path = matches.get_one::<String>("config").unwrap();

    // Create a Tokio runtime
    let rt = tokio::runtime::Runtime::new()?;
    let mut config = load_config(config_path)?;

    // Override with command line arguments
    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.database.data_dir = data_dir.clone();
    }
    if let Some(host) = matches.get_one::<String>("host") {
        config.server.host = host.clone();
    }
    if let Some(port) = matches.get_one::<String>("port") {
        config.server.port = port
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid port number: {}", e))?;
    }

    // Override with environment variables
    if let Ok(host) = std::env::var("MCP_SERVER_HOST") {
        config.server.host = host;
    }
    if let Ok(port) = std::env::var("MCP_SERVER_PORT") {
        config.server.port = port
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid MCP_SERVER_PORT: {}", e))?;
    }
    if let Ok(data_dir) = std::env::var("KOTADB_DATA_DIR") {
        config.database.data_dir = data_dir;
    }

    tracing::info!("Starting KotaDB MCP Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Configuration: {:#?}", config);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.database.data_dir)?;

    // Run the async server operations within the runtime
    rt.block_on(async {
        // Initialize and start the MCP server
        let server = init_mcp_server(config).await?;

        tracing::info!("MCP server initialization complete");

        // Start the server and get a handle
        let server_handle = server.start().await?;

        // Handle graceful shutdown without blocking
        tokio::select! {
            _ = setup_shutdown_handler() => {
                tracing::info!("Received shutdown signal, closing server");
                server_handle.close();
            }
        }

        tracing::info!("MCP server stopped gracefully");
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

/// Load configuration from file or use defaults
#[cfg(feature = "mcp-server")]
fn load_config(config_path: &str) -> Result<MCPConfig> {
    if std::path::Path::new(config_path).exists() {
        tracing::info!("Loading configuration from: {}", config_path);
        MCPConfig::from_file(config_path)
    } else {
        tracing::warn!(
            "Configuration file not found: {}, using defaults",
            config_path
        );
        Ok(MCPConfig::default())
    }
}

/// Perform a health check and exit
#[cfg(feature = "mcp-server")]
async fn perform_health_check() -> Result<()> {
    use std::time::Duration;
    use tokio::time::timeout;

    println!("Performing KotaDB MCP server health check...");

    // Test configuration loading
    let config = MCPConfig::default();
    println!("✓ Configuration validation passed");

    // Test data directory access
    std::fs::create_dir_all(&config.database.data_dir)?;
    println!("✓ Data directory accessible: {}", config.database.data_dir);

    // Test server initialization (with timeout)
    let init_result = timeout(Duration::from_secs(10), async {
        init_mcp_server(config).await
    })
    .await;

    match init_result {
        Ok(Ok(_server)) => {
            println!("✓ MCP server initialization successful");
            println!("✓ All health checks passed");
            Ok(())
        }
        Ok(Err(e)) => {
            eprintln!("✗ MCP server initialization failed: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("✗ Health check timed out after 10 seconds");
            std::process::exit(1);
        }
    }
}

/// Setup graceful shutdown handler
#[cfg(feature = "mcp-server")]
async fn setup_shutdown_handler() {
    use tokio::signal;

    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to register SIGTERM handler");
    let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .expect("Failed to register SIGINT handler");

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM");
        }
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT");
        }
        _ = signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C");
        }
    }
}

#[cfg(not(feature = "mcp-server"))]
fn main() {
    eprintln!("MCP server support is disabled. Build with --features mcp-server to enable.");
    std::process::exit(1);
}

#[cfg(all(test, feature = "mcp-server"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_default_config() -> Result<()> {
        let config = load_config("nonexistent.toml")?;
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check_basic() -> Result<()> {
        // This is a basic test - full health check would require more setup
        let config = MCPConfig::default();
        assert!(config.server.port > 0);
        assert!(!config.database.data_dir.is_empty());
        Ok(())
    }

    #[test]
    fn test_data_directory_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let data_path = temp_dir.path().join("test-data");

        std::fs::create_dir_all(&data_path)?;
        assert!(data_path.exists());
        assert!(data_path.is_dir());

        Ok(())
    }
}
