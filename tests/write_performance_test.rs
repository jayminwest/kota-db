// Write Performance Test - Validates fix for Issue #151
// Tests that write operations maintain consistent performance without significant outliers

use anyhow::Result;
use kotadb::{
    builders::DocumentBuilder,
    contracts::Storage,
    file_storage::create_file_storage,
    metrics::write_performance::{WriteMetricsConfig, WritePerformanceMonitor},
};
use std::time::{Duration, Instant};
use tempfile::TempDir;
mod test_constants;
use test_constants::{gating, performance as perf};

/// Performance requirements based on Issue #151
struct PerformanceRequirements {
    max_avg_latency_ms: u64,     // Average should be under 10ms
    max_p95_latency_ms: u64,     // P95 should be under 50ms
    max_p99_latency_ms: u64,     // P99 should be under 100ms
    max_std_dev_ms: u64,         // Standard deviation should be under 25ms
    max_outlier_percentage: f64, // Max 5% outliers allowed
}

impl Default for PerformanceRequirements {
    fn default() -> Self {
        Self {
            max_avg_latency_ms: perf::write_avg_ms(),
            max_p95_latency_ms: perf::write_p95_ms(),
            max_p99_latency_ms: perf::write_p99_ms(),
            max_std_dev_ms: perf::write_stddev_ms(),
            max_outlier_percentage: perf::write_outlier_pct(),
        }
    }
}

#[tokio::test]
async fn test_write_performance_consistency() -> Result<()> {
    if gating::skip_if_heavy_disabled("write_performance_test::test_write_performance_consistency")
    {
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;

    // Create performance monitor
    let monitor = WritePerformanceMonitor::new(WriteMetricsConfig {
        window_size: 1000,
        outlier_threshold_ms: 50,
        log_outliers: true,
    });

    // Perform 100 write operations and measure latencies
    let num_operations = 100;
    for i in 0..num_operations {
        let doc = DocumentBuilder::new()
            .path(format!("perf_test_{}.md", i))
            .unwrap()
            .title(format!("Performance Test {}", i))
            .unwrap()
            .content(format!("Test content for document {}", i).as_bytes())
            .build()
            .unwrap();

        let start = Instant::now();
        storage.insert(doc).await?;
        let duration = start.elapsed();

        monitor.record_write(duration).await;
    }

    // Get performance statistics
    let stats = monitor.get_stats().await;
    let requirements = PerformanceRequirements::default();

    // Log results for debugging
    println!("\n=== Write Performance Results ===");
    println!("Operations: {}", stats.count);
    println!("Average: {:?}", stats.avg_duration);
    println!("Median: {:?}", stats.median_duration);
    println!("Min: {:?}", stats.min_duration.unwrap_or_default());
    println!("Max: {:?}", stats.max_duration.unwrap_or_default());
    println!("P95: {:?}", stats.p95_duration);
    println!("P99: {:?}", stats.p99_duration);
    println!("Std Dev: {:?}", stats.std_deviation);
    println!(
        "Outliers: {} ({:.2}%)",
        stats.outlier_count,
        (stats.outlier_count as f64 / stats.count as f64) * 100.0
    );

    // Verify performance requirements
    assert!(
        stats.avg_duration.as_millis() as u64 <= requirements.max_avg_latency_ms,
        "Average latency {:?} exceeds requirement of {}ms",
        stats.avg_duration,
        requirements.max_avg_latency_ms
    );

    assert!(
        stats.p95_duration.as_millis() as u64 <= requirements.max_p95_latency_ms,
        "P95 latency {:?} exceeds requirement of {}ms",
        stats.p95_duration,
        requirements.max_p95_latency_ms
    );

    assert!(
        stats.p99_duration.as_millis() as u64 <= requirements.max_p99_latency_ms,
        "P99 latency {:?} exceeds requirement of {}ms",
        stats.p99_duration,
        requirements.max_p99_latency_ms
    );

    assert!(
        stats.std_deviation.as_millis() as u64 <= requirements.max_std_dev_ms,
        "Standard deviation {:?} exceeds requirement of {}ms",
        stats.std_deviation,
        requirements.max_std_dev_ms
    );

    let outlier_percentage = (stats.outlier_count as f64 / stats.count as f64) * 100.0;
    assert!(
        outlier_percentage <= requirements.max_outlier_percentage,
        "Outlier percentage {:.2}% exceeds requirement of {:.2}%",
        outlier_percentage,
        requirements.max_outlier_percentage
    );

    Ok(())
}

#[tokio::test]
async fn test_write_buffering_effectiveness() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(100)).await?;

    // Measure time for a batch of writes
    let batch_size = 50;
    let mut documents = Vec::new();

    for i in 0..batch_size {
        documents.push(
            DocumentBuilder::new()
                .path(format!("batch_test_{}.md", i))
                .unwrap()
                .title(format!("Batch Test {}", i))
                .unwrap()
                .content(b"Test content")
                .build()
                .unwrap(),
        );
    }

    // Measure batch write time
    let start = Instant::now();
    for doc in documents {
        storage.insert(doc).await?;
    }
    let batch_duration = start.elapsed();

    // Force flush to ensure all writes are persisted
    storage.flush().await?;

    let avg_per_doc = batch_duration / batch_size as u32;
    println!("\nBatch write performance:");
    println!("  Total time for {} docs: {:?}", batch_size, batch_duration);
    println!("  Average per document: {:?}", avg_per_doc);

    // Buffering should keep average write time under a CI-aware threshold
    let max_avg_ms = perf::write_avg_ms();
    assert!(
        avg_per_doc.as_millis() < max_avg_ms as u128,
        "Buffered write average {:?} should be under {}ms",
        avg_per_doc,
        max_avg_ms
    );

    Ok(())
}

#[tokio::test]
async fn test_write_performance_under_load() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000)).await?;

    let monitor = WritePerformanceMonitor::new(WriteMetricsConfig::default());

    // Simulate sustained write load
    let duration = Duration::from_secs(2);
    let start_time = Instant::now();
    let mut operation_count = 0;

    while start_time.elapsed() < duration {
        let doc = DocumentBuilder::new()
            .path(format!("load_test_{}.md", operation_count))
            .unwrap()
            .title(format!("Load Test {}", operation_count))
            .unwrap()
            .content(format!("Content {}", operation_count).as_bytes())
            .build()
            .unwrap();

        let op_start = Instant::now();
        storage.insert(doc).await?;
        let op_duration = op_start.elapsed();

        monitor.record_write(op_duration).await;
        operation_count += 1;
    }

    // Final flush
    storage.flush().await?;

    let stats = monitor.get_stats().await;
    let throughput = operation_count as f64 / duration.as_secs_f64();

    println!("\n=== Sustained Load Test Results ===");
    println!("Duration: {:?}", duration);
    println!("Operations: {}", operation_count);
    println!("Throughput: {:.2} ops/sec", throughput);
    println!("Average latency: {:?}", stats.avg_duration);
    println!("P99 latency: {:?}", stats.p99_duration);

    // Should maintain reasonable throughput (CI-aware)
    let min_tput = if test_constants::concurrency::is_ci() {
        80.0
    } else {
        100.0
    };
    assert!(
        throughput >= min_tput,
        "Throughput {:.2} ops/sec should be at least {:.0} ops/sec",
        throughput,
        min_tput
    );

    // P99 should stay under threshold even under load
    let max_p99_ms = perf::write_p99_ms();
    assert!(
        stats.p99_duration.as_millis() < max_p99_ms as u128,
        "P99 latency {:?} should be under {}ms even under load",
        stats.p99_duration,
        max_p99_ms
    );

    Ok(())
}

#[tokio::test]
async fn test_outlier_detection() -> Result<()> {
    // Test that our monitoring correctly identifies outliers
    let monitor = WritePerformanceMonitor::new(WriteMetricsConfig {
        window_size: 100,
        outlier_threshold_ms: 10,
        log_outliers: false,
    });

    // Normal operations
    for _ in 0..10 {
        monitor.record_write(Duration::from_millis(5)).await;
    }

    // Outliers
    monitor.record_write(Duration::from_millis(50)).await;
    monitor.record_write(Duration::from_millis(100)).await;

    let outliers = monitor.get_outliers().await;
    assert_eq!(outliers.len(), 2, "Should detect 2 outliers");

    let stats = monitor.get_stats().await;
    assert_eq!(stats.outlier_count, 2);

    Ok(())
}
