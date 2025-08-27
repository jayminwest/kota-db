//! Test binary symbol format performance

use anyhow::Result;
use kotadb::git::{IngestionConfig, RepositoryIngester};
use std::path::PathBuf;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("kotadb=info".parse()?),
        )
        .init();

    println!("🚀 Testing Binary Symbol Format Performance");
    println!("{}", "=".repeat(50));

    // Get repository path from args or use current directory
    let repo_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Set up paths
    let storage_path = PathBuf::from("./data/binary-test/storage");
    let symbol_db_path = PathBuf::from("./data/binary-test/symbols.kota");

    // Create directories
    std::fs::create_dir_all(&storage_path)?;
    std::fs::create_dir_all(symbol_db_path.parent().unwrap())?;

    // Create storage
    let mut storage =
        kotadb::file_storage::create_file_storage(storage_path.to_str().unwrap(), Some(100))
            .await?;

    // Configure ingestion with symbol extraction enabled
    let mut config = IngestionConfig::default();
    config.options.extract_symbols = true;

    // Create ingester
    let ingester = RepositoryIngester::new(config);

    // Run ingestion with binary symbols
    println!("\n📁 Ingesting repository: {}", repo_path.display());
    let start = Instant::now();

    let result = ingester
        .ingest_with_binary_symbols(
            &repo_path,
            &mut storage,
            &symbol_db_path,
            Some(Box::new(|msg| println!("  {}", msg))),
        )
        .await?;

    let elapsed = start.elapsed();

    // Print results
    println!("\n✅ Ingestion Complete!");
    println!("{}", "=".repeat(50));
    println!("📊 Results:");
    println!("  Documents created: {}", result.documents_created);
    println!("  Files ingested: {}", result.files_ingested);
    println!("  Symbols extracted: {}", result.symbols_extracted);
    println!("  Files with symbols: {}", result.files_with_symbols);
    println!("  Errors: {}", result.errors);
    println!("\n⏱️  Total time: {:?}", elapsed);
    println!(
        "  Average: {:.2}ms per file",
        elapsed.as_millis() as f64 / result.files_ingested.max(1) as f64
    );

    // Test reading back
    println!("\n🔍 Testing symbol read performance...");
    let read_start = Instant::now();

    let reader = kotadb::binary_symbols::BinarySymbolReader::open(&symbol_db_path)?;
    println!("  Opened database with {} symbols", reader.symbol_count());

    // Read first 10 symbols
    for i in 0..10.min(reader.symbol_count()) {
        if let Some(symbol) = reader.get_symbol(i) {
            let name = reader.get_symbol_name(&symbol)?;
            let file = reader.get_symbol_file_path(&symbol)?;
            println!("  Symbol {}: {} in {}", i, name, file);
        }
    }

    let read_elapsed = read_start.elapsed();
    println!("\n  Read time: {:?}", read_elapsed);

    Ok(())
}
