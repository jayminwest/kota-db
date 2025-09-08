// E2E Integration Tests - KotaDB Testing Pyramid Phase 2
// End-to-End test suite implementing the missing 5% layer of the testing pyramid
// Tests complete user journeys and workflow validation

use anyhow::Result;

mod e2e;
use e2e::*;

/// Integration test that validates the E2E test framework itself
#[tokio::test]
async fn test_e2e_framework_setup() -> Result<()> {
    let env = TestEnvironment::new()?;
    env.setup_test_codebase()?;

    // Verify test environment setup
    assert!(env.db_path().exists());
    assert!(env.codebase_path().exists());
    assert!(env.codebase_path().join("Cargo.toml").exists());
    assert!(env.codebase_path().join("src/lib.rs").exists());

    // Verify project root detection
    let cargo_toml = env.project_root().join("Cargo.toml");
    assert!(cargo_toml.exists());

    println!("✅ E2E framework setup validation passed");

    Ok(())
}

/// Integration test that validates CommandRunner functionality
#[tokio::test]
async fn test_command_runner_functionality() -> Result<()> {
    let env = TestEnvironment::new()?;
    env.ensure_binary_built().await?;

    let runner = CommandRunner::new(&env);

    // Test help command (should be fast and always work)
    let help_result = runner.run(&["--help"]).await?;

    runner.validate_success(&help_result)?;
    runner.validate_performance(&help_result, 1000)?; // Help should be <1s
    runner.validate_output_contains(&help_result, "KotaDB")?;

    println!("✅ CommandRunner functionality validation passed");

    Ok(())
}
