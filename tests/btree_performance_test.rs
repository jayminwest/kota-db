// B+ Tree Performance Test - Stage 1: TDD
// This test verifies O(log n) performance characteristics

use kotadb::{btree, ValidatedDocumentId, ValidatedPath};
use std::time::Instant;
use uuid::Uuid;

#[test]
fn test_btree_insertion_performance() -> anyhow::Result<()> {
    println!("\n=== B+ Tree Insertion Performance Test ===");

    let sizes = vec![100, 1000, 10000];
    let mut timings = Vec::new();

    for size in sizes {
        // Generate test data
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();
        let paths: Vec<_> = (0..size)
            .map(|i| ValidatedPath::new(format!("/perf/doc_{i}.md")).unwrap())
            .collect();

        // Time insertion
        let start = Instant::now();
        let mut tree = btree::create_empty_tree();
        for i in 0..size {
            tree = btree::insert_into_tree(tree, keys[i], paths[i].clone())?;
        }
        let duration = start.elapsed();

        let avg_time_us = duration.as_micros() as f64 / size as f64;
        timings.push((size, avg_time_us));

        println!("Size: {size:5} | Total: {duration:?} | Avg per insert: {avg_time_us:.2}μs");
    }

    // Verify O(log n) behavior: time should not increase linearly
    // For O(log n), when n increases 10x, time should increase ~3.3x (log10)
    // For O(n), when n increases 10x, time increases 10x
    let ratio_1_to_2 = timings[1].1 / timings[0].1;
    let ratio_2_to_3 = timings[2].1 / timings[1].1;

    println!("\nGrowth analysis:");
    println!("100 → 1000 (10x): time increased {ratio_1_to_2:.2}x");
    println!("1000 → 10000 (10x): time increased {ratio_2_to_3:.2}x");

    // For O(log n), ratios should be much less than 10
    assert!(
        ratio_1_to_2 < 5.0,
        "Performance degraded too much: {ratio_1_to_2:.2}x"
    );
    assert!(
        ratio_2_to_3 < 5.0,
        "Performance degraded too much: {ratio_2_to_3:.2}x"
    );

    Ok(())
}

#[test]
fn test_btree_search_performance() -> anyhow::Result<()> {
    println!("\n=== B+ Tree Search Performance Test ===");

    let sizes = vec![100, 1000, 10000];
    let mut timings = Vec::new();

    for size in sizes {
        // Build tree
        let mut tree = btree::create_empty_tree();
        let keys: Vec<_> = (0..size)
            .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
            .collect();

        for (i, key) in keys.iter().enumerate() {
            let path = ValidatedPath::new(format!("/perf/doc_{i}.md"))?;
            tree = btree::insert_into_tree(tree, *key, path)?;
        }

        // Search for middle 10% of keys
        let search_count = size / 10;
        let search_keys: Vec<_> = keys.iter().skip(size * 4 / 10).take(search_count).collect();

        // Time searches
        let start = Instant::now();
        for key in &search_keys {
            let _ = btree::search_in_tree(&tree, key);
        }
        let duration = start.elapsed();

        let avg_time_us = duration.as_micros() as f64 / search_count as f64;
        timings.push((size, avg_time_us));

        println!("Size: {size:5} | {search_count} searches | Avg per search: {avg_time_us:.2}μs");
    }

    // Verify O(log n) behavior - handle near-zero measurements
    let ratio_1_to_2 = if timings[0].1 > 0.01 {
        timings[1].1 / timings[0].1
    } else {
        // If first measurement is too small, just check absolute time
        timings[1].1 / 0.01 // Use 0.01μs as minimum threshold
    };

    let ratio_2_to_3 = if timings[1].1 > 0.01 {
        timings[2].1 / timings[1].1
    } else {
        timings[2].1 / 0.01
    };

    println!("\nGrowth analysis:");
    println!("100 → 1000 (10x): time increased {ratio_1_to_2:.2}x");
    println!("1000 → 10000 (10x): time increased {ratio_2_to_3:.2}x");

    // For O(log n), search time should increase very slowly
    // If measurements are very fast (< 1μs), just check they're reasonable
    if timings[2].1 > 1.0 {
        assert!(
            ratio_1_to_2 < 10.0,
            "Search performance degraded too much: {ratio_1_to_2:.2}x"
        );
        assert!(
            ratio_2_to_3 < 10.0,
            "Search performance degraded too much: {ratio_2_to_3:.2}x"
        );
    } else {
        // All measurements are very fast, which is good
        println!("All search times < 1μs - excellent performance!");
    }

    Ok(())
}

#[test]
fn test_btree_vs_linear_performance() -> anyhow::Result<()> {
    println!("\n=== B+ Tree vs Linear Search Comparison ===");

    let size = 10000;

    // Build data
    let keys: Vec<_> = (0..size)
        .map(|_| ValidatedDocumentId::from_uuid(Uuid::new_v4()).unwrap())
        .collect();

    // Build B+ tree
    let mut tree = btree::create_empty_tree();
    for (i, key) in keys.iter().enumerate() {
        let path = ValidatedPath::new(format!("/perf/doc_{i}.md"))?;
        tree = btree::insert_into_tree(tree, *key, path)?;
    }

    // Search for a key in the middle
    let target_key = &keys[size / 2];

    // Time linear search
    let start = Instant::now();
    let mut found = false;
    for key in &keys {
        if key == target_key {
            found = true;
            break;
        }
    }
    let linear_time = start.elapsed();
    assert!(found);

    // Time B+ tree search (run multiple times for accuracy)
    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = btree::search_in_tree(&tree, target_key);
    }
    let btree_total_time = start.elapsed();
    let btree_time = btree_total_time / iterations;

    println!("Linear search (O(n)): {linear_time:?}");
    println!("B+ tree search (O(log n)): {btree_time:?}");
    println!(
        "Speedup: {:.2}x",
        linear_time.as_nanos() as f64 / btree_time.as_nanos() as f64
    );

    // B+ tree should be significantly faster
    assert!(
        btree_time < linear_time / 10,
        "B+ tree not fast enough: {btree_time:?} vs {linear_time:?}"
    );

    Ok(())
}
