//! KotaDB SaaS API Server
//!
//! Production HTTP server with API key authentication,
//! rate limiting, and codebase intelligence features.

use anyhow::Result;
use clap::Parser;
use kotadb::{create_file_storage, start_saas_server, ApiKeyConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "KotaDB SaaS API Server")]
struct Args {
    /// Data directory path
    #[arg(short = 'd', long, default_value = "/data", env = "KOTADB_DATA_DIR")]
    data_dir: PathBuf,

    /// Server port
    #[arg(short = 'p', long, default_value = "8080", env = "PORT")]
    port: u16,

    /// PostgreSQL database URL for API keys
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Maximum database connections
    #[arg(long, default_value = "10", env = "DATABASE_MAX_CONNECTIONS")]
    max_connections: u32,

    /// Database connection timeout in seconds
    #[arg(long, default_value = "30", env = "DATABASE_CONNECT_TIMEOUT")]
    connect_timeout: u64,

    /// Default rate limit (requests per minute)
    #[arg(long, default_value = "60", env = "DEFAULT_RATE_LIMIT")]
    default_rate_limit: u32,

    /// Default monthly quota (requests per month)
    #[arg(long, default_value = "1000000", env = "DEFAULT_MONTHLY_QUOTA")]
    default_monthly_quota: u64,

    /// Enable quiet mode (minimal logging)
    #[arg(short = 'q', long, env = "QUIET_MODE")]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first with proper RUST_LOG environment variable support
    // For the API server, we always want to respect RUST_LOG environment variable
    if let Err(e) = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
    {
        // Logging might already be initialized, that's ok
        eprintln!("Note: Logging initialization returned: {:?}", e);
    }

    info!("🔧 Parsing command line arguments...");

    // Check environment variables before parsing
    info!(
        "DATABASE_URL env var present: {}",
        std::env::var("DATABASE_URL").is_ok()
    );
    info!("PORT env var: {:?}", std::env::var("PORT"));
    info!(
        "KOTADB_DATA_DIR env var: {:?}",
        std::env::var("KOTADB_DATA_DIR")
    );

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Failed to parse arguments: {}", e);
            info!("Argument parsing error: {}", e);
            return Err(anyhow::anyhow!("Failed to parse arguments: {}", e));
        }
    };

    info!("🚀 Starting KotaDB SaaS API Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Data directory: {}", args.data_dir.display());
    info!("Port: {}", args.port);
    info!("Database URL configured: {}", !args.database_url.is_empty());

    info!("📁 Creating data directory...");
    std::fs::create_dir_all(&args.data_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create data directory: {}", e))?;

    info!("💾 Initializing storage backend...");
    let storage_path = args.data_dir.join("storage");
    let storage = create_file_storage(
        storage_path.to_str().unwrap(),
        Some(1000), // Cache capacity
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create storage: {}", e))?;
    let storage = Arc::new(Mutex::new(storage));

    info!("🔑 Configuring API key service...");
    let api_key_config = ApiKeyConfig {
        database_url: args.database_url.clone(),
        max_connections: args.max_connections,
        connect_timeout_seconds: args.connect_timeout,
        default_rate_limit: args.default_rate_limit,
        default_monthly_quota: args.default_monthly_quota,
    };

    info!("🔍 Testing database connectivity...");
    info!("Database URL configured: {}", !args.database_url.is_empty());
    info!("Max connections: {}", args.max_connections);
    info!("Connect timeout: {}s", args.connect_timeout);

    // Test database connection before starting server
    match kotadb::test_database_connection(&api_key_config).await {
        Ok(_) => {
            info!("✅ Database connection successful");
        }
        Err(e) => {
            eprintln!("❌ Database connection failed: {}", e);
            info!("❌ Database connection failed: {}", e);
            return Err(anyhow::anyhow!("Database connection failed: {}", e));
        }
    }

    info!("🚀 Starting server on port {}...", args.port);
    match start_saas_server(storage, args.data_dir, api_key_config, args.port).await {
        Ok(_) => {
            info!("✅ Server started successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to start server: {}", e);
            info!("❌ Failed to start server: {}", e);
            Err(anyhow::anyhow!("Failed to start server: {}", e))
        }
    }
}
