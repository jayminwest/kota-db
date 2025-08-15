// Write Performance Metrics - Monitoring for Issue #151
// Tracks write operation latencies to identify and diagnose performance outliers

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::observability::{record_metric, MetricType};

/// Configuration for write performance monitoring
#[derive(Debug, Clone)]
pub struct WriteMetricsConfig {
    /// Size of the sliding window for latency tracking
    pub window_size: usize,
    /// Threshold for outlier detection (in milliseconds)
    pub outlier_threshold_ms: u64,
    /// Whether to log outliers
    pub log_outliers: bool,
}

impl Default for WriteMetricsConfig {
    fn default() -> Self {
        Self {
            window_size: 1000,        // Track last 1000 operations
            outlier_threshold_ms: 50, // Flag writes over 50ms as outliers
            log_outliers: true,
        }
    }
}

/// Statistics for write operations
#[derive(Debug, Clone, Default)]
pub struct WriteStats {
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub avg_duration: Duration,
    pub median_duration: Duration,
    pub p95_duration: Duration,
    pub p99_duration: Duration,
    pub std_deviation: Duration,
    pub outlier_count: u64,
}

/// Tracks write operation performance metrics
pub struct WritePerformanceMonitor {
    config: WriteMetricsConfig,
    latencies: Arc<RwLock<VecDeque<Duration>>>,
    outlier_operations: Arc<RwLock<Vec<OutlierOperation>>>,
    total_operations: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
struct OutlierOperation {
    duration: Duration,
    timestamp: std::time::Instant,
    operation_id: u64,
}

impl WritePerformanceMonitor {
    /// Create a new write performance monitor
    pub fn new(config: WriteMetricsConfig) -> Self {
        let window_size = config.window_size;
        Self {
            config,
            latencies: Arc::new(RwLock::new(VecDeque::with_capacity(window_size))),
            outlier_operations: Arc::new(RwLock::new(Vec::new())),
            total_operations: Arc::new(RwLock::new(0)),
        }
    }

    /// Record a write operation's duration
    pub async fn record_write(&self, duration: Duration) {
        let mut latencies = self.latencies.write().await;
        let mut total_ops = self.total_operations.write().await;

        *total_ops += 1;
        let operation_id = *total_ops;

        // Maintain sliding window
        if latencies.len() >= self.config.window_size {
            latencies.pop_front();
        }
        latencies.push_back(duration);

        // Check for outlier
        let duration_ms = duration.as_millis() as u64;
        if duration_ms > self.config.outlier_threshold_ms {
            let outlier = OutlierOperation {
                duration,
                timestamp: std::time::Instant::now(),
                operation_id,
            };

            if self.config.log_outliers {
                warn!(
                    "Write operation outlier detected: operation {} took {:?} (threshold: {}ms)",
                    operation_id, duration, self.config.outlier_threshold_ms
                );
            }

            let mut outliers = self.outlier_operations.write().await;
            outliers.push(outlier);

            // Emit outlier metric
            record_metric(MetricType::Counter {
                name: "storage.write.outliers",
                value: 1,
            });
        }

        // Emit standard metrics
        record_metric(MetricType::Histogram {
            name: "storage.write.latency",
            value: duration.as_millis() as f64,
            unit: "ms",
        });
    }

    /// Get current write statistics
    pub async fn get_stats(&self) -> WriteStats {
        let latencies = self.latencies.read().await;
        let outliers = self.outlier_operations.read().await;
        let total_ops = *self.total_operations.read().await;

        if latencies.is_empty() {
            return WriteStats::default();
        }

        // Convert to sorted vector for percentile calculations
        let mut sorted_latencies: Vec<Duration> = latencies.iter().copied().collect();
        sorted_latencies.sort();

        let count = sorted_latencies.len();
        let total: Duration = sorted_latencies.iter().sum();
        let avg = total / count as u32;

        // Calculate percentiles
        let median_idx = count / 2;
        let p95_idx = (count as f64 * 0.95) as usize;
        let p99_idx = (count as f64 * 0.99) as usize;

        let median = sorted_latencies[median_idx];
        let p95 = sorted_latencies[p95_idx.min(count - 1)];
        let p99 = sorted_latencies[p99_idx.min(count - 1)];

        // Calculate standard deviation
        let variance: f64 = sorted_latencies
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - avg.as_secs_f64();
                diff * diff
            })
            .sum::<f64>()
            / count as f64;

        let std_dev = Duration::from_secs_f64(variance.sqrt());

        WriteStats {
            count: total_ops,
            total_duration: total,
            min_duration: sorted_latencies.first().copied(),
            max_duration: sorted_latencies.last().copied(),
            avg_duration: avg,
            median_duration: median,
            p95_duration: p95,
            p99_duration: p99,
            std_deviation: std_dev,
            outlier_count: outliers.len() as u64,
        }
    }

    /// Get detailed information about outliers
    pub async fn get_outliers(&self) -> Vec<(Duration, std::time::Instant, u64)> {
        let outliers = self.outlier_operations.read().await;
        outliers
            .iter()
            .map(|o| (o.duration, o.timestamp, o.operation_id))
            .collect()
    }

    /// Clear all collected metrics
    pub async fn reset(&self) {
        *self.latencies.write().await = VecDeque::with_capacity(self.config.window_size);
        self.outlier_operations.write().await.clear();
        *self.total_operations.write().await = 0;
    }

    /// Log a summary of current performance
    pub async fn log_summary(&self) {
        let stats = self.get_stats().await;

        info!(
            "Write Performance Summary - Operations: {}, Avg: {:?}, Median: {:?}, P95: {:?}, P99: {:?}, StdDev: {:?}, Outliers: {}",
            stats.count,
            stats.avg_duration,
            stats.median_duration,
            stats.p95_duration,
            stats.p99_duration,
            stats.std_deviation,
            stats.outlier_count
        );

        if stats.outlier_count > 0 {
            let outlier_percentage = (stats.outlier_count as f64 / stats.count as f64) * 100.0;
            warn!(
                "Write performance degradation: {:.2}% of operations exceeded {}ms threshold",
                outlier_percentage, self.config.outlier_threshold_ms
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_metrics_tracking() {
        let monitor = WritePerformanceMonitor::new(WriteMetricsConfig {
            window_size: 10,
            outlier_threshold_ms: 10,
            log_outliers: false,
        });

        // Record normal operations
        for _ in 0..5 {
            monitor.record_write(Duration::from_millis(5)).await;
        }

        // Record outliers
        for _ in 0..2 {
            monitor.record_write(Duration::from_millis(50)).await;
        }

        let stats = monitor.get_stats().await;
        assert_eq!(stats.count, 7);
        assert_eq!(stats.outlier_count, 2);
        assert!(stats.avg_duration > Duration::from_millis(5));
    }

    #[tokio::test]
    async fn test_percentile_calculations() {
        let monitor = WritePerformanceMonitor::new(WriteMetricsConfig::default());

        // Create a distribution of latencies
        for i in 1..=100 {
            monitor.record_write(Duration::from_millis(i)).await;
        }

        let stats = monitor.get_stats().await;

        // Median should be around 50ms
        assert!(stats.median_duration >= Duration::from_millis(49));
        assert!(stats.median_duration <= Duration::from_millis(51));

        // P95 should be around 95ms
        assert!(stats.p95_duration >= Duration::from_millis(94));
        assert!(stats.p95_duration <= Duration::from_millis(96));

        // P99 should be around 99ms
        assert!(stats.p99_duration >= Duration::from_millis(98));
        assert!(stats.p99_duration <= Duration::from_millis(100));
    }
}
