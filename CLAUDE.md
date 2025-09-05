# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Branching Strategy

This repository uses Git Flow. **Always work on feature branches, never directly on main or develop.**

### Quick Reference
- **Current branch to work from**: `develop`
- **Create feature branches**: `git checkout -b feature/your-feature`
- **Create PRs to**: `develop` branch
- **Production branch**: `main` (protected, requires reviews)

### Workflow
```bash
# Start new work
git checkout develop
git pull origin develop
git checkout -b feature/your-feature

# After making changes
git add .
git commit -m "feat: your changes"
git push -u origin feature/your-feature

# Create PR
gh pr create --base develop
```

For more details, see `docs/BRANCHING_STRATEGY.md`.

## GitHub Issue Management

### Label Management Protocol
Before creating issues, always check existing labels and create new ones if needed:

```bash
# Check what labels are available
gh label list --limit 100

# Search for specific label types
gh label list --search "bug"
gh label list --search "performance"

# Create new labels when needed
gh label create "database" --description "Database-related issues" --color "1d76db"
gh label create "embedding" --description "Embedding and vector search issues" --color "0e8a16"
gh label create "mcp-server" --description "MCP server related issues" --color "6f42c1"

# Create issues with appropriate labels
gh issue create --title "Issue title" --body "Description" --label "bug,storage,priority-high"
```

### Standard Label Categories for KotaDB
- **Component**: `storage`, `index`, `mcp`, `embedding`, `vector-search`, `trigram`, `primary-index`
- **Type**: `bug`, `enhancement`, `feature`, `refactor`, `documentation`, `test`  
- **Priority**: `priority-critical`, `priority-high`, `priority-medium`, `priority-low`
- **Effort**: `effort-small` (< 1 day), `effort-medium` (1-3 days), `effort-large` (> 3 days)
- **Status**: `needs-investigation`, `blocked`, `in-progress`, `ready-for-review`

## Commands for Development

### Versioning and Release

**🚨 IMPORTANT: Always perform releases from the `develop` branch!**

```bash
# STEP 1: Switch to develop branch
git checkout develop
git pull origin develop

# STEP 2: Check current version and preview release
just version                # Current version
just release-preview        # Shows unreleased changes and recent commits

# STEP 3: Run release command (FROM DEVELOP BRANCH)
just release-patch          # Bump patch: 0.1.0 -> 0.1.1
just release-minor          # Bump minor: 0.1.0 -> 0.2.0
just release-major          # Bump major: 0.1.0 -> 1.0.0
just release-beta           # Beta release: 0.1.0 -> 0.1.0-beta.1

# Or release specific version
just release 0.2.0          # Full release process

# STEP 4: After release, merge main back to develop
git fetch origin main
git merge origin/main -m "chore: sync version updates from v[VERSION] release"
git push origin develop

# Other commands
just release-dry-run 0.2.0  # Test without making changes
just changelog-update       # Add new unreleased section after release
```

The release process automatically:
- Runs all tests and quality checks
- Updates version in Cargo.toml, VERSION, CHANGELOG.md
- Updates client library versions
- Creates git tag with changelog
- Pushes to main branch
- Triggers GitHub Actions for binaries, Docker images, crates.io, PyPI, and npm

### Installation Requirements for Optimal Development

For the fastest development experience, install these tools:

```bash
# REQUIRED for fast testing (3-5x speed improvement)
cargo install cargo-nextest --locked

# OPTIONAL for file watching and auto-reload (may have platform issues)
cargo install cargo-watch --locked  # Note: May fail on some macOS configurations

# REQUIRED for development workflow
cargo install just              # Task runner (if not available system-wide)
```

### Build and Run
```bash
# Build the project
cargo build
cargo build --release  # Production build

# Run codebase intelligence commands
cargo run --bin kotadb -- -d ./kota-db-data stats              # Show database statistics
cargo run --bin kotadb -- -d ./kota-db-data search-code "rust" # Full-text code search (<3ms)
cargo run --bin kotadb -- -d ./kota-db-data search-symbols "*" # Wildcard symbol search

# Codebase indexing with symbol extraction
cargo run --bin kotadb -- -d ./kota-db-data index-codebase .           # Index repository with symbols
cargo run --bin kotadb -- -d ./kota-db-data stats --symbols               # Check extracted symbols
cargo run --bin kotadb -- -d ./kota-db-data find-callers FileStorage   # Find who calls a function
cargo run --bin kotadb -- -d ./kota-db-data analyze-impact Config      # Analyze change impact

# Performance benchmarking
cargo run --release -- benchmark --operations 10000   # Run performance benchmarks
cargo run --release -- benchmark -t storage -o 5000   # Benchmark storage operations
cargo run --release -- benchmark -f json              # Output results as JSON

# Development server with auto-reload
just dev                     # Uses cargo watch for auto-reload
```

### Testing (FAST - 3-5x Speed Improvement)
```bash
# FAST: Run all tests with cargo-nextest (3-5x faster than standard cargo test)
cargo nextest run --all
just test                                  # Now uses cargo-nextest by default

# FAST: Run specific test types  
cargo nextest run --lib                    # Unit tests only (FAST)
cargo nextest run --test '*'               # Integration tests only (FAST)
just test-unit                             # Unit tests (FAST)
just test-integration                      # Integration tests (FAST)

# Legacy commands (for compatibility/fallback)
cargo test --all                           # Legacy - slower
just test-legacy                           # Legacy - slower
just test-fast                             # Explicit fast testing

# Performance and stress tests (still use cargo test for benchmarking)
cargo test --release --features bench performance_regression_test
just test-perf

# Run single test with output
cargo nextest run test_name                # FAST single test
cargo test test_name -- --nocapture        # Legacy with output

# Watch mode (if cargo-watch is available)
just watch                                 # Auto-runs tests on file changes with nextest
```

### Code Quality
```bash
# Format code
cargo fmt --all
just fmt

# Run clippy linter (MUST pass with zero warnings)
cargo clippy --all-targets --all-features -- -D warnings
just clippy

# Run all quality checks (format check + clippy + unit tests)
just check
```

### Benchmarking
```bash
# Run database benchmarks
just db-bench
cargo run --release -- benchmark --operations 10000

# Run performance benchmarks
cargo bench --features bench
```

## High-Level Architecture

KotaDB is a codebase intelligence platform that helps AI assistants understand code relationships, dependencies, and structure. Built in Rust with zero external database dependencies.

### Core Components

#### Storage Layer (`src/file_storage.rs`)
- **FileStorage**: Page-based storage engine with Write-Ahead Log (WAL)
- **Binary Storage**: High-performance binary format for symbols and relationships (10x faster)
- **Dual Architecture**: Separates code content from relationship data for optimal performance
- **Persistence**: 4KB page-based architecture with checksums

#### Index Systems
1. **Primary Index** (`src/primary_index.rs`)
   - B+ tree implementation for O(log n) path-based lookups
   - Handles wildcard queries and range scans
   - Full persistence with crash recovery

2. **Trigram Index** (`src/trigram_index.rs`)
   - Full-text search using trigram tokenization
   - Dual-index architecture: trigram → documents and document → trigrams
   - Fuzzy search tolerance with ranking

3. **Vector Index** (`src/vector_index.rs`)
   - HNSW (Hierarchical Navigable Small World) for vector similarity search
   - Supports embeddings from multiple providers

#### Wrapper System (`src/wrappers.rs`)
Production-ready wrappers that compose around storage and indices:
- **TracedStorage**: Distributed tracing with unique IDs
- **ValidatedStorage**: Runtime contract validation
- **RetryableStorage**: Automatic retry with exponential backoff
- **CachedStorage**: LRU caching for frequently accessed documents
- **MeteredIndex**: Performance metrics and monitoring

#### Type Safety (`src/types.rs`, `src/validation.rs`)
Validated types ensure compile-time and runtime safety:
- `ValidatedPath`, `ValidatedDocumentId`, `ValidatedTimestamp`
- Builder patterns for safe construction
- Comprehensive validation rules

#### Query System
- **Code Intelligence**: Find callers, analyze impact, track dependencies
- **Symbol Search**: Fast pattern-based symbol discovery (functions, classes, variables)
- **Relationship Queries**: Find function calls, dependencies, and usage patterns
- **Performance**: Sub-10ms query latency for code analysis operations

### MCP Server (`src/mcp/`)
Model Context Protocol server for LLM integration:
- `src/bin/mcp_server.rs`: Full MCP server implementation
- `src/bin/mcp_server_minimal.rs`: Minimal implementation for testing
- Configuration via `kotadb-mcp-dev.toml`

### Testing Infrastructure
- **Unit Tests**: In-module tests throughout codebase
- **Integration Tests**: `tests/` directory with comprehensive scenarios
- **Performance Tests**: `benches/` directory with criterion benchmarks
- **Stress Tests**: Chaos testing, concurrent access, adversarial inputs
- **Test Constants**: `tests/test_constants.rs` for shared test configuration

### Key Design Patterns

1. **6-Stage Risk Reduction**: Test-driven, contract-first, pure functions, observability, adversarial testing, component library
2. **Builder Pattern**: Safe construction of complex types (DocumentBuilder, QueryBuilder)
3. **Factory Functions**: `create_*` functions return fully-wrapped production components
4. **Async-First**: All I/O operations use async/await with Tokio
5. **Zero-Copy**: Extensive use of references and memory-mapped I/O

## Working with the Codebase

### Adding New Features
1. Start with tests in the appropriate test file
2. Implement using existing patterns and wrappers
3. Ensure all tests pass including integration tests
4. Run `just check` to verify code quality

### Dogfooding for Validation - MANDATORY Practice

**🚨 CRITICAL: Always dogfood your changes on KotaDB's own codebase before submitting PRs.**

When working on search, indexing, MCP, or git features, testing on KotaDB itself is required, not optional. This practice has prevented every major integration bug from reaching production.

#### Current Dogfooding (CLI-based)
```bash
# ALWAYS start with fresh setup for accurate testing
rm -rf data/analysis && mkdir -p data/analysis

# Core dogfooding commands - USE THESE CONSTANTLY
cargo run --bin kotadb -- -d ./data/analysis index-codebase .             # Index with symbols (default enabled)
cargo run --bin kotadb -- -d ./data/analysis stats --symbols               # Verify extraction
cargo run --bin kotadb -- -d ./data/analysis search-code "async fn"        # Test content search  
cargo run --bin kotadb -- -d ./data/analysis search-symbols "Storage"      # Test symbol search
cargo run --bin kotadb -- -d ./data/analysis find-callers FileStorage      # Test relationships
cargo run --bin kotadb -- -d ./data/analysis analyze-impact Config         # Test impact analysis

# Performance validation - MEASURE REAL LATENCY
time cargo run --release --bin kotadb -- -d ./data/analysis search-code "rust"
cargo run --release -- benchmark --operations 1000  # Compare to baseline
```

#### Future Dogfooding (API/MCP Integration)
```bash
# MCP server dogfooding (connect Claude Code once available)
cargo run --bin mcp_server --config kotadb-dev.toml &
# Test through Claude Code:
# - Real AI assistant query patterns
# - Concurrent request handling  
# - Long-running sessions
# - Memory usage under AI load

# HTTP API dogfooding (future capability)
cargo run --release -- server --port 8080 &
# Integration testing through API endpoints
```

#### Mandatory Dogfooding Protocol

**Before starting work:**
- Fresh index of KotaDB codebase in `data/analysis/`
- Baseline performance measurements
- Verify all core operations work correctly

**During development:**  
- Re-test after every significant change
- Use realistic queries an AI assistant would generate
- Monitor performance impact continuously
- Test edge cases with actual code complexity

**Before PR submission:**
- Complete re-index and validation
- Performance regression testing  
- Edge case validation (malformed queries, empty results, large datasets)
- Create GitHub issues for any problems discovered

#### Proven Bug Detection Record

Real-world testing on KotaDB consistently reveals integration issues that unit tests miss:
- **Issue #191**: Search disconnection after git ingestion (dogfooding only)
- **Issue #196**: Trigram index architectural limitation (self-analysis discovery)
- **Issue #184**: Multiple UX and functionality gaps (comprehensive validation)
- **Issue #179**: Symbol extraction edge cases with complex Rust code
- **Issue #203**: Performance degradation under realistic query patterns
- **Issue #157**: Memory usage issues only visible with large codebases

**Key insight**: Every major integration bug has been caught through dogfooding, demonstrating its critical importance.

#### Dogfooding Best Practices

**Directory management:**
- **Use separate directories**: `data/analysis/` for testing, `data/test-scenarios/` for specific cases
- **Clean up thoroughly**: Delete all analysis data when done (never commit artifacts)
- **Preserve baselines**: Keep `kota-db-data/` for normal usage if needed

**Testing thoroughness:**
- Test queries that AI assistants actually use
- Validate performance meets <10ms targets
- Verify symbol extraction >95% accuracy
- Test concurrent access patterns
- Validate incremental update behavior

**Issue reporting:**
When dogfooding reveals problems, immediately create GitHub issues:
```bash
gh issue create --title "[Dogfooding] Found: [description]" \
  --body "Found during dogfooding test. Scenario: [details]. Impact: [analysis]" \
  --label "bug,dogfooding,priority-high"
```

This systematic approach maintains KotaDB's 99% reliability while enabling rapid AI-first feature development.

### Performance Considerations
- Use `MeteredIndex` wrapper for new indices
- Leverage connection pooling for concurrent operations
- Profile with `cargo bench` before and after changes
- Target sub-10ms query latency

### Common Patterns
- Use factory functions (`create_*`) instead of direct construction
- Wrap all storage/index implementations with appropriate wrappers
- Use validated types for all user input
- Include tracing spans for observability

### Error Handling
- Use `anyhow::Result` for application errors
- Use `thiserror` for library errors
- Always include context with `.context()`
- Log errors at appropriate levels with `tracing`

## Important Files

### Versioning & Release
- `VERSION` - Current version number (plain text)
- `CHANGELOG.md` - Version history following Keep a Changelog format
- `scripts/release.sh` - Automated release script
- `scripts/version-bump.sh` - Version bumping utility
- `docs/RELEASE_PROCESS.md` - Complete release documentation
- `.github/workflows/release.yml` - GitHub Actions release automation

Always update CHANGELOG.md when making user-facing changes by adding entries to the `[Unreleased]` section.