// Optimization Metrics - Stage 4: Observability for Phase 2 Infrastructure
// Enhanced metrics for bulk operations and concurrent access patterns

use crate::contracts::optimization::{
    BulkOperationResult, BulkOperationType, ContentionMetrics, OptimizationRecommendation,
    TreeStructureMetrics,
};
use crate::metrics::performance::{PerformanceCollector, PerformanceDashboard};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Optimization metrics collector extending performance metrics
#[derive(Debug)]
pub struct OptimizationMetricsCollector {
    performance_collector: PerformanceCollector,
    bulk_operation_history: Arc<RwLock<Vec<BulkOperationMetric>>>,
    contention_tracker: Arc<Mutex<ContentionTracker>>,
    tree_analysis_cache: Arc<RwLock<Option<CachedTreeAnalysis>>>,
    configuration: OptimizationMetricsConfig,
}

/// Configuration for optimization metrics collection
#[derive(Debug, Clone)]
pub struct OptimizationMetricsConfig {
    pub track_bulk_operations: bool,
    pub track_contention: bool,
    pub tree_analysis_interval: Duration,
    pub max_bulk_history: usize,
    pub contention_sample_rate: f64, // 0.0 to 1.0
    pub enable_realtime_recommendations: bool,
}

impl Default for OptimizationMetricsConfig {
    fn default() -> Self {
        Self {
            track_bulk_operations: true,
            track_contention: true,
            tree_analysis_interval: Duration::from_secs(300), // 5 minutes
            max_bulk_history: 1000,
            contention_sample_rate: 0.1, // 10% sampling
            enable_realtime_recommendations: true,
        }
    }
}

/// Individual bulk operation metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationMetric {
    pub operation_type: BulkOperationType,
    pub timestamp: SystemTime,
    pub input_size: usize,
    pub result: BulkOperationResult,
    pub efficiency_score: f64,  // 0.0 to 1.0
    pub speedup_factor: f64,    // vs individual operations
    pub memory_efficiency: f64, // data size / memory used
}

/// Lock contention tracking
#[derive(Debug)]
struct ContentionTracker {
    read_locks: u32,
    write_locks: u32,
    pending_reads: u32,
    pending_writes: u32,
    total_lock_acquisitions: u64,
    contested_acquisitions: u64,
    lock_wait_times: Vec<Duration>,
    last_reset: Instant,
}

impl Default for ContentionTracker {
    fn default() -> Self {
        Self {
            read_locks: 0,
            write_locks: 0,
            pending_reads: 0,
            pending_writes: 0,
            total_lock_acquisitions: 0,
            contested_acquisitions: 0,
            lock_wait_times: Vec::new(),
            last_reset: Instant::now(),
        }
    }
}

/// Cached tree analysis with expiration
#[derive(Debug, Clone)]
struct CachedTreeAnalysis {
    metrics: TreeStructureMetrics,
    timestamp: Instant,
    ttl: Duration,
}

/// Comprehensive optimization dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationDashboard {
    pub timestamp: SystemTime,
    pub performance_dashboard: PerformanceDashboard,
    pub bulk_operations: BulkOperationSummary,
    pub contention_metrics: ContentionMetrics,
    pub tree_analysis: TreeStructureMetrics,
    pub efficiency_trends: EfficiencyTrends,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub compliance_status: SLAComplianceStatus,
}

/// Summary of bulk operations performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationSummary {
    pub total_operations: u64,
    pub avg_efficiency_score: f64,
    pub avg_speedup_factor: f64,
    pub operations_by_type: HashMap<String, BulkOperationTypeStats>,
    pub recent_operations: Vec<BulkOperationMetric>,
}

/// Statistics for a specific bulk operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkOperationTypeStats {
    pub count: u64,
    pub avg_efficiency: f64,
    pub avg_speedup: f64,
    pub avg_input_size: usize,
    pub success_rate: f64,
    pub last_operation: Option<SystemTime>,
}

/// Efficiency trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyTrends {
    pub bulk_efficiency_trend: Vec<EfficiencyDataPoint>,
    pub memory_efficiency_trend: Vec<EfficiencyDataPoint>,
    pub contention_trend: Vec<EfficiencyDataPoint>,
    pub tree_balance_trend: Vec<EfficiencyDataPoint>,
}

/// Single efficiency measurement point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyDataPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub context: String,
}

/// SLA compliance status across all optimization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAComplianceStatus {
    pub overall_compliant: bool,
    pub bulk_operations_compliant: bool,
    pub contention_compliant: bool,
    pub tree_health_compliant: bool,
    pub violations: Vec<String>,
    pub compliance_score: f64, // 0.0 to 1.0
}

impl OptimizationMetricsCollector {
    pub fn new(config: OptimizationMetricsConfig) -> Self {
        let performance_config = crate::metrics::performance::PerformanceConfig::default();

        Self {
            performance_collector: PerformanceCollector::new(performance_config),
            bulk_operation_history: Arc::new(RwLock::new(Vec::new())),
            contention_tracker: Arc::new(Mutex::new(ContentionTracker::default())),
            tree_analysis_cache: Arc::new(RwLock::new(None)),
            configuration: config,
        }
    }

    /// Record a bulk operation for analysis
    pub fn record_bulk_operation(
        &self,
        operation_type: BulkOperationType,
        input_size: usize,
        result: BulkOperationResult,
        baseline_duration: Option<Duration>, // For speedup calculation
    ) -> anyhow::Result<()> {
        if !self.configuration.track_bulk_operations {
            return Ok(());
        }

        // Calculate efficiency metrics
        let efficiency_score = self.calculate_efficiency_score(&result);
        let speedup_factor = if let Some(baseline) = baseline_duration {
            baseline.as_secs_f64() / result.duration.as_secs_f64()
        } else {
            1.0
        };

        let memory_efficiency = if result.memory_delta_bytes > 0 {
            // Estimate based on input size and memory delta
            (input_size * 32) as f64 / result.memory_delta_bytes as f64
        } else {
            1.0
        };

        let metric = BulkOperationMetric {
            operation_type,
            timestamp: SystemTime::now(),
            input_size,
            result,
            efficiency_score,
            speedup_factor,
            memory_efficiency,
        };

        // Store metric
        let mut history = self.bulk_operation_history.write();
        history.push(metric);

        // Maintain history size
        if history.len() > self.configuration.max_bulk_history {
            history.remove(0);
        }

        Ok(())
    }

    /// Record lock contention event
    pub fn record_lock_contention(
        &self,
        lock_type: LockType,
        wait_time: Duration,
        was_contested: bool,
    ) {
        if !self.configuration.track_contention {
            return;
        }

        // Sample based on configuration
        if fastrand::f64() > self.configuration.contention_sample_rate {
            return;
        }

        let mut tracker = match self.contention_tracker.lock() {
            Ok(t) => t,
            Err(_) => return, // Lock poisoned, skip recording
        };

        match lock_type {
            LockType::Read => {
                if was_contested {
                    tracker.pending_reads = tracker.pending_reads.saturating_sub(1);
                }
                tracker.read_locks = tracker.read_locks.saturating_add(1);
            }
            LockType::Write => {
                if was_contested {
                    tracker.pending_writes = tracker.pending_writes.saturating_sub(1);
                }
                tracker.write_locks = tracker.write_locks.saturating_add(1);
            }
        }

        tracker.total_lock_acquisitions += 1;
        if was_contested {
            tracker.contested_acquisitions += 1;
        }

        tracker.lock_wait_times.push(wait_time);

        // Keep only recent wait times (last 1000)
        if tracker.lock_wait_times.len() > 1000 {
            tracker.lock_wait_times.remove(0);
        }
    }

    /// Record pending lock request
    pub fn record_lock_pending(&self, lock_type: LockType) {
        if !self.configuration.track_contention {
            return;
        }

        let mut tracker = match self.contention_tracker.lock() {
            Ok(t) => t,
            Err(_) => return, // Lock poisoned, skip recording
        };
        match lock_type {
            LockType::Read => tracker.pending_reads += 1,
            LockType::Write => tracker.pending_writes += 1,
        }
    }

    /// Update tree analysis cache
    pub fn update_tree_analysis(&self, metrics: TreeStructureMetrics) {
        let mut cache = self.tree_analysis_cache.write();
        *cache = Some(CachedTreeAnalysis {
            metrics,
            timestamp: Instant::now(),
            ttl: self.configuration.tree_analysis_interval,
        });
    }

    /// Generate comprehensive optimization dashboard
    pub fn generate_optimization_dashboard(&self) -> OptimizationDashboard {
        let performance_dashboard = self.performance_collector.generate_dashboard();
        let bulk_operations = self.generate_bulk_operation_summary();
        let contention_metrics = self.generate_contention_metrics();
        let tree_analysis = self
            .get_cached_tree_analysis()
            .unwrap_or_else(|| self.generate_default_tree_metrics());
        let efficiency_trends = self.generate_efficiency_trends();
        let recommendations = self.generate_recommendations(&tree_analysis, &contention_metrics);
        let compliance_status = self.assess_sla_compliance(&bulk_operations, &contention_metrics);

        OptimizationDashboard {
            timestamp: SystemTime::now(),
            performance_dashboard,
            bulk_operations,
            contention_metrics,
            tree_analysis,
            efficiency_trends,
            recommendations,
            compliance_status,
        }
    }

    /// Calculate efficiency score for a bulk operation
    fn calculate_efficiency_score(&self, result: &BulkOperationResult) -> f64 {
        let mut score: f64 = 1.0;

        // Penalize errors
        if !result.errors.is_empty() {
            score *= 0.5;
        }

        // Reward high throughput
        if result.throughput_ops_per_sec > 10000.0 {
            score *= 1.2;
        } else if result.throughput_ops_per_sec < 1000.0 {
            score *= 0.8;
        }

        // Reward good tree balance
        if result.tree_balance_factor > 0.9 {
            score *= 1.1;
        } else if result.tree_balance_factor < 0.7 {
            score *= 0.9;
        }

        // Reward efficient complexity
        score *= match result.complexity_class {
            crate::contracts::performance::ComplexityClass::Constant => 1.3,
            crate::contracts::performance::ComplexityClass::Logarithmic => 1.2,
            crate::contracts::performance::ComplexityClass::Linearithmic => 1.0,
            crate::contracts::performance::ComplexityClass::Linear => 0.8,
            crate::contracts::performance::ComplexityClass::Quadratic => 0.5,
            crate::contracts::performance::ComplexityClass::Unknown => 0.7,
        };

        score.clamp(0.0, 1.0)
    }

    /// Generate bulk operation summary
    fn generate_bulk_operation_summary(&self) -> BulkOperationSummary {
        let history = self.bulk_operation_history.read();

        if history.is_empty() {
            return BulkOperationSummary {
                total_operations: 0,
                avg_efficiency_score: 0.0,
                avg_speedup_factor: 0.0,
                operations_by_type: HashMap::new(),
                recent_operations: Vec::new(),
            };
        }

        let total_operations = history.len() as u64;
        let avg_efficiency_score =
            history.iter().map(|m| m.efficiency_score).sum::<f64>() / history.len() as f64;
        let avg_speedup_factor =
            history.iter().map(|m| m.speedup_factor).sum::<f64>() / history.len() as f64;

        // Group by operation type
        let mut operations_by_type: HashMap<String, BulkOperationTypeStats> = HashMap::new();

        for metric in history.iter() {
            let type_name = format!("{:?}", metric.operation_type);
            let stats =
                operations_by_type
                    .entry(type_name)
                    .or_insert_with(|| BulkOperationTypeStats {
                        count: 0,
                        avg_efficiency: 0.0,
                        avg_speedup: 0.0,
                        avg_input_size: 0,
                        success_rate: 0.0,
                        last_operation: None,
                    });

            stats.count += 1;
            stats.avg_efficiency += metric.efficiency_score;
            stats.avg_speedup += metric.speedup_factor;
            stats.avg_input_size += metric.input_size;
            if metric.result.errors.is_empty() {
                stats.success_rate += 1.0;
            }
            stats.last_operation = Some(metric.timestamp);
        }

        // Finalize averages
        for stats in operations_by_type.values_mut() {
            if stats.count > 0 {
                stats.avg_efficiency /= stats.count as f64;
                stats.avg_speedup /= stats.count as f64;
                stats.avg_input_size /= stats.count as usize;
                stats.success_rate /= stats.count as f64;
            }
        }

        // Get recent operations (last 10)
        let recent_operations = history.iter().rev().take(10).cloned().collect();

        BulkOperationSummary {
            total_operations,
            avg_efficiency_score,
            avg_speedup_factor,
            operations_by_type,
            recent_operations,
        }
    }

    /// Generate contention metrics
    fn generate_contention_metrics(&self) -> ContentionMetrics {
        let tracker = match self.contention_tracker.lock() {
            Ok(t) => t,
            Err(_) => {
                // Lock poisoned, return empty metrics
                return ContentionMetrics {
                    active_readers: 0,
                    active_writers: 0,
                    pending_readers: 0,
                    pending_writers: 0,
                    read_lock_wait_time: Duration::ZERO,
                    write_lock_wait_time: Duration::ZERO,
                    lock_acquisition_rate: 0.0,
                    contention_ratio: 0.0,
                };
            }
        };

        let contention_ratio = if tracker.total_lock_acquisitions > 0 {
            tracker.contested_acquisitions as f64 / tracker.total_lock_acquisitions as f64
        } else {
            0.0
        };

        let avg_wait_time = if !tracker.lock_wait_times.is_empty() {
            let total: Duration = tracker.lock_wait_times.iter().sum();
            total / tracker.lock_wait_times.len() as u32
        } else {
            Duration::ZERO
        };

        let lock_acquisition_rate = if tracker.last_reset.elapsed() > Duration::ZERO {
            tracker.total_lock_acquisitions as f64 / tracker.last_reset.elapsed().as_secs_f64()
        } else {
            0.0
        };

        ContentionMetrics {
            active_readers: tracker.read_locks,
            active_writers: tracker.write_locks,
            pending_readers: tracker.pending_reads,
            pending_writers: tracker.pending_writes,
            read_lock_wait_time: avg_wait_time,
            write_lock_wait_time: avg_wait_time, // Simplified - could track separately
            lock_acquisition_rate,
            contention_ratio,
        }
    }

    /// Get cached tree analysis or None if expired
    fn get_cached_tree_analysis(&self) -> Option<TreeStructureMetrics> {
        let cache = self.tree_analysis_cache.read();
        if let Some(ref cached) = *cache {
            if cached.timestamp.elapsed() < cached.ttl {
                return Some(cached.metrics.clone());
            }
        }
        None
    }

    /// Generate default tree metrics when none are cached
    fn generate_default_tree_metrics(&self) -> TreeStructureMetrics {
        // Return placeholder metrics - in practice, this would trigger a tree analysis
        TreeStructureMetrics {
            total_entries: 0,
            tree_depth: 0,
            balance_factor: 1.0,
            utilization_factor: 0.0,
            memory_efficiency: 0.0,
            node_distribution: crate::contracts::optimization::NodeDistribution {
                total_nodes: 0,
                leaf_nodes: 0,
                internal_nodes: 0,
                avg_keys_per_node: 0.0,
                min_keys_per_node: 0,
                max_keys_per_node: 0,
            },
            leaf_depth_variance: 0,
            recommended_actions: Vec::new(),
        }
    }

    /// Generate efficiency trends
    fn generate_efficiency_trends(&self) -> EfficiencyTrends {
        let history = self.bulk_operation_history.read();

        let bulk_efficiency_trend: Vec<_> = history
            .iter()
            .take(50) // Last 50 operations
            .map(|m| EfficiencyDataPoint {
                timestamp: m.timestamp,
                value: m.efficiency_score,
                context: format!("{:?}", m.operation_type),
            })
            .collect();

        let memory_efficiency_trend: Vec<_> = history
            .iter()
            .take(50)
            .map(|m| EfficiencyDataPoint {
                timestamp: m.timestamp,
                value: m.memory_efficiency,
                context: format!("size_{}", m.input_size),
            })
            .collect();

        // Simplified trends - in practice, would track more comprehensive data
        EfficiencyTrends {
            bulk_efficiency_trend,
            memory_efficiency_trend,
            contention_trend: Vec::new(),
            tree_balance_trend: Vec::new(),
        }
    }

    /// Generate optimization recommendations
    fn generate_recommendations(
        &self,
        tree_metrics: &TreeStructureMetrics,
        contention_metrics: &ContentionMetrics,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = tree_metrics.recommended_actions.clone();

        // Add contention-based recommendations
        if contention_metrics.contention_ratio > 0.3 {
            recommendations.push(OptimizationRecommendation::OptimizeBulkOperations {
                operation_type: "concurrent_access".to_string(),
                current_efficiency: 1.0 - contention_metrics.contention_ratio,
                target_efficiency: 0.9,
            });
        }

        if contention_metrics.pending_writers > 5 {
            recommendations.push(OptimizationRecommendation::EnableCaching {
                hot_paths: vec!["write_operations".to_string()],
                estimated_speedup: 2.0,
            });
        }

        recommendations
    }

    /// Assess SLA compliance across all metrics
    fn assess_sla_compliance(
        &self,
        bulk_summary: &BulkOperationSummary,
        contention_metrics: &ContentionMetrics,
    ) -> SLAComplianceStatus {
        let mut violations = Vec::new();
        let mut compliant_components = 0;
        let total_components = 3;

        // Check bulk operations compliance
        let bulk_compliant =
            bulk_summary.avg_efficiency_score >= 0.8 && bulk_summary.avg_speedup_factor >= 5.0;
        if bulk_compliant {
            compliant_components += 1;
        } else {
            violations.push("Bulk operations below efficiency thresholds".to_string());
        }

        // Check contention compliance
        let contention_compliant =
            contention_metrics.contention_ratio < 0.3 && contention_metrics.pending_writers < 10;
        if contention_compliant {
            compliant_components += 1;
        } else {
            violations.push("Lock contention exceeds acceptable limits".to_string());
        }

        // Check tree health (simplified)
        let tree_compliant = true; // Would check tree metrics
        if tree_compliant {
            compliant_components += 1;
        }

        let compliance_score = compliant_components as f64 / total_components as f64;
        let overall_compliant = compliance_score >= 0.8;

        SLAComplianceStatus {
            overall_compliant,
            bulk_operations_compliant: bulk_compliant,
            contention_compliant,
            tree_health_compliant: tree_compliant,
            violations,
            compliance_score,
        }
    }

    /// Export optimization metrics to JSON
    pub fn export_optimization_json(&self) -> String {
        let dashboard = self.generate_optimization_dashboard();
        serde_json::to_string_pretty(&dashboard).unwrap_or_else(|_| "{}".to_string())
    }

    /// Export optimization metrics to Prometheus format
    pub fn export_optimization_prometheus(&self) -> String {
        let dashboard = self.generate_optimization_dashboard();
        let mut prometheus = String::new();

        // Bulk operation metrics
        prometheus.push_str(&format!(
            "kotadb_bulk_operations_total {}\n",
            dashboard.bulk_operations.total_operations
        ));
        prometheus.push_str(&format!(
            "kotadb_bulk_efficiency_avg {:.3}\n",
            dashboard.bulk_operations.avg_efficiency_score
        ));
        prometheus.push_str(&format!(
            "kotadb_bulk_speedup_avg {:.1}\n",
            dashboard.bulk_operations.avg_speedup_factor
        ));

        // Contention metrics
        prometheus.push_str(&format!(
            "kotadb_lock_contention_ratio {:.3}\n",
            dashboard.contention_metrics.contention_ratio
        ));
        prometheus.push_str(&format!(
            "kotadb_active_readers {}\n",
            dashboard.contention_metrics.active_readers
        ));
        prometheus.push_str(&format!(
            "kotadb_active_writers {}\n",
            dashboard.contention_metrics.active_writers
        ));

        // Tree health metrics
        prometheus.push_str(&format!(
            "kotadb_tree_balance_factor {:.3}\n",
            dashboard.tree_analysis.balance_factor
        ));
        prometheus.push_str(&format!(
            "kotadb_tree_utilization {:.3}\n",
            dashboard.tree_analysis.utilization_factor
        ));

        // Compliance metrics
        prometheus.push_str(&format!(
            "kotadb_sla_compliance_score {:.3}\n",
            dashboard.compliance_status.compliance_score
        ));

        prometheus
    }
}

/// Type of lock for contention tracking
#[derive(Debug, Clone, Copy)]
pub enum LockType {
    Read,
    Write,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_optimization_metrics_collector() {
        let config = OptimizationMetricsConfig {
            contention_sample_rate: 1.0, // 100% sampling for reliable tests
            ..Default::default()
        };
        let collector = OptimizationMetricsCollector::new(config);

        // Record a bulk operation
        let result = BulkOperationResult::success(1000, Duration::from_millis(100), 1024, 0.95);
        let _ = collector.record_bulk_operation(
            BulkOperationType::Insert,
            1000,
            result,
            Some(Duration::from_secs(1)), // Baseline for speedup calculation
        );

        // Record contention
        collector.record_lock_contention(LockType::Write, Duration::from_millis(50), true);

        // Generate dashboard
        let dashboard = collector.generate_optimization_dashboard();

        assert!(dashboard.bulk_operations.total_operations > 0);
        assert!(dashboard.bulk_operations.avg_efficiency_score > 0.0);
        assert!(dashboard.contention_metrics.contention_ratio > 0.0);
    }

    #[test]
    fn test_efficiency_score_calculation() {
        let config = OptimizationMetricsConfig::default();
        let collector = OptimizationMetricsCollector::new(config);

        // Test high-efficiency operation
        let good_result = BulkOperationResult {
            operations_completed: 1000,
            duration: Duration::from_millis(50),
            throughput_ops_per_sec: 20000.0,
            memory_delta_bytes: 1024,
            tree_balance_factor: 0.95,
            complexity_class: crate::contracts::performance::ComplexityClass::Logarithmic,
            errors: Vec::new(),
        };

        let score = collector.calculate_efficiency_score(&good_result);
        assert!(score > 0.8, "Expected high efficiency score, got {score}");

        // Test low-efficiency operation
        let bad_result = BulkOperationResult {
            operations_completed: 100,
            duration: Duration::from_secs(1),
            throughput_ops_per_sec: 100.0,
            memory_delta_bytes: 10240,
            tree_balance_factor: 0.5,
            complexity_class: crate::contracts::performance::ComplexityClass::Quadratic,
            errors: vec!["Test error".to_string()],
        };

        let score = collector.calculate_efficiency_score(&bad_result);
        assert!(score < 0.5, "Expected low efficiency score, got {score}");
    }
}
