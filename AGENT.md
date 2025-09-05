# AGENT.md - Essential Guide for LLM Coding Agents

> **🤖 This codebase is developed and maintained 100% by LLM coding tools.**  
> **Agents communicate exclusively through GitHub issues, PRs, and commits.**

## 🚨 CRITICAL: Agent Communication Protocol

### 1. GitHub CLI is MANDATORY
All agents MUST use the GitHub CLI (`gh`) for ALL interactions:
```bash
# ALWAYS comment on issues when working on them
gh issue comment <issue-number> --body "Starting work on this issue. Current status: [details]"

# ALWAYS comment on commits with context
gh api repos/:owner/:repo/commits/<sha>/comments --method POST --field body="[Agent] This commit addresses [issue]. Impact: [details]"

# ALWAYS comment on PRs extensively
gh pr comment <pr-number> --body "Code review complete. Found [details]. Suggestions: [list]"

# ALWAYS check available labels before creating issues
gh label list --limit 100  # Review existing labels first

# Create new labels if needed before making issues
gh label create "new-label" --description "Label description" --color "E99695"

# Create issues with appropriate labels
gh issue create --title "[Agent] Found issue: [description]" --body "[Detailed description with context]" --label "bug,priority-high"
```

### 2. Agent Handoff Protocol
When a new agent takes over:
1. **Read latest GitHub issues** - This is your primary source of truth
2. **Check recent PR comments** - Understand what other agents have done
3. **Comment on relevant issues** - Announce you're taking over
4. **Update progress liberally** - Other agents depend on your updates

### 3. Documentation Requirements
EVERY agent action must be accompanied by:
- **Issue comments** explaining what you're doing and why
- **Commit messages** following conventional commits format
- **PR descriptions** with detailed impact analysis
- **Code comments** only when logic is complex (prefer self-documenting code)
- **CHANGELOG.md updates** for any user-facing changes (add to Unreleased section)

### 4. Documentation Location Priority
**ALWAYS prefer GitHub over creating .md files:**
- **Issues** - For tracking work, problems, and feature requests
- **PR descriptions** - For implementation details and decisions
- **GitHub Discussions** - For architecture decisions and design questions
- **GitHub Wiki** - For persistent documentation that doesn't fit elsewhere
- **Comments on commits** - For explaining why changes were made

**❌ AVOID creating .md files in root directory** unless absolutely necessary for project structure (like README.md, CONTRIBUTING.md). Use GitHub's native documentation features instead.

### 4a. Label Management Protocol
**ALWAYS check and manage labels appropriately:**

```bash
# Check existing labels before creating issues
gh label list --search "bug"        # Search for specific label types
gh label list --limit 100           # List all labels (increase limit if needed)

# Create new labels when appropriate
gh label create "enhancement" --description "New feature or improvement" --color "84b6eb"
gh label create "security" --description "Security-related issues" --color "d73a4a"
gh label create "performance" --description "Performance optimization" --color "0052cc"
gh label create "documentation" --description "Documentation improvements" --color "0075ca"

# Apply labels when creating issues
gh issue create --title "Fix memory leak" --body "Details..." --label "bug,priority-high,performance"

# Add labels to existing issues
gh issue edit 123 --add-label "security,critical"
```

**Standard Label Categories:**
- **Type**: `bug`, `enhancement`, `feature`, `documentation`
- **Priority**: `priority-critical`, `priority-high`, `priority-medium`, `priority-low`
- **Status**: `in-progress`, `blocked`, `needs-review`, `ready-to-merge`
- **Component**: `storage`, `index`, `mcp`, `testing`, `ci-cd`
- **Effort**: `effort-small`, `effort-medium`, `effort-large`

### 5. Efficient Agent Operations
**Use subagents liberally to optimize context usage:**
- **Spawn subagents** for independent tasks to reduce main context size
- **Delegate specific files** to subagents for focused work (e.g., "fix all tests in file X")
- **Run parallel subagents** for tasks that don't interfere with each other
- **Use subagents for research** - let them read documentation and report back summaries
- **Delegate repetitive work** - let subagents handle similar patterns across multiple files

**IMPORTANT**: To run subagents truly in parallel, you must call multiple subagents in the same message using multiple tool invocations. Sequential messages will run subagents one after another.

This keeps your main context focused on high-level coordination and decision-making.

## 🏗️ Project Overview

**KotaDB** is a codebase intelligence platform that helps AI assistants understand code relationships, dependencies, and structure. Built in Rust with zero external database dependencies.

### Key Facts
- **Language**: Rust (edition 2021)
- **Repository**: https://github.com/jayminwest/kota-db
- **Purpose**: Reduce LLM context usage by 70% while improving code understanding
- **Architecture**: 6-stage risk reduction methodology (99% success rate)
- **Performance**: Sub-10ms query latency for code analysis

## 🎯 Current Status & Priorities

### ✅ COMPLETED (DO NOT BREAK)
- **Codebase Intelligence Core** - Symbol extraction, dependency analysis, impact analysis
- **Index Systems** - B+ tree, trigram search (<3ms), HNSW vector, relationship graph
- **Binary Storage Pipeline** - 10x faster than JSON for symbol operations
- **CI/CD Pipeline** - Comprehensive testing and deployment automation

### 🔄 ACTIVE DEVELOPMENT AREAS
- **MCP Integration** - Seamless Claude Code integration via Model Context Protocol
- **Query Performance** - Optimize for AI assistant usage patterns
- **Relationship Queries** - Enhanced "who calls what" and impact analysis
- **Symbol Search** - Advanced pattern matching and code navigation

### 📋 UPCOMING PHASES
- One-click MCP setup for Claude Code
- GitHub auto-sync for repositories
- Advanced code intelligence features
- Support for more AI assistants

## 🌳 Branching Strategy (Git Flow)

**CRITICAL**: All development must follow our Git Flow branching model.

### Branch Structure
```
feature/* ──┐
            ├──> develop ──> release/* ──> main
hotfix/*  ──────────────────────────────┘
```

### Where to Work
- **New features**: Create `feature/*` branches from `develop`
- **Bug fixes**: Create `feature/*` branches from `develop`
- **Emergency fixes**: Create `hotfix/*` branches from `main`
- **NEVER**: Push directly to `main` or `develop`

### Workflow for Agents
```bash
# 1. Start new work
git checkout develop
git pull origin develop
git checkout -b feature/your-feature-name

# 2. Make changes and commit
git add .
git commit -m "feat: describe your change"

# 3. Push and create PR
git push -u origin feature/your-feature-name
gh pr create --base develop --title "feat: your feature"

# 4. After merge, clean up
git checkout develop
git pull origin develop
git branch -d feature/your-feature-name
```

### Branch Protection Rules
- **main**: Requires PR, review, all CI checks, up-to-date
- **develop**: Requires PR, CI checks (no review needed)

See `docs/BRANCHING_STRATEGY.md` for complete details.

## 🛠️ Development Commands

### Essential Commands (Use these frequently)
```bash
# Primary development workflow
just dev              # Start development server with auto-reload
just test              # Run all tests (REQUIRED before commits)
just check             # Run all quality checks (formatting, linting, tests)
just ci                # Run full CI pipeline locally

# Testing specific areas
just test-unit         # Unit tests only
just test-integration  # Integration tests only
just test-perf         # Performance regression tests
just coverage          # Generate test coverage report

# Code quality (REQUIRED)
just fmt               # Format code (run before every commit)
just clippy            # Linting (must pass with no warnings)
just audit             # Security audit (run weekly)

# Documentation
just docs              # Build and open API documentation
just docs-serve        # Serve docs on localhost:8000

# Database operations
just demo              # Run Stage 6 demo (shows component library in action)
just db-bench          # Performance benchmarks

# Container development
just docker-up         # Start development containers
just docker-shell      # Connect to development container
```

### Standalone Execution
```bash
# Alternative to `just` commands
./run_standalone.sh status   # Project status
./run_standalone.sh test     # Run tests
./run_standalone.sh demo     # Stage 6 demo
./run_standalone.sh build    # Build project
```

## 🔬 Dogfooding - MANDATORY for Development

**🚨 CRITICAL: Always dogfood KotaDB on its own codebase when working on core features.**

When working on search, indexing, MCP, or git features, you MUST test on KotaDB itself. This is not optional - it's how we prevent deployment issues.

### Current Testing Methods (CLI)
```bash
# Primary dogfooding setup - USE THIS CONSTANTLY
mkdir -p data/analysis  # Separate directory for testing
cargo run --bin kotadb -- -d ./data/analysis index-codebase .

# Essential validation commands - RUN THESE FREQUENTLY
cargo run --bin kotadb -- -d ./data/analysis stats --symbols        # Verify symbol extraction
cargo run --bin kotadb -- -d ./data/analysis search-code "storage"  # Test content search
cargo run --bin kotadb -- -d ./data/analysis search-symbols "*"     # Test symbol search
cargo run --bin kotadb -- -d ./data/analysis find-callers Storage   # Test relationship queries
cargo run --bin kotadb -- -d ./data/analysis analyze-impact Config  # Test impact analysis

# Performance validation - MEASURE EVERYTHING
time cargo run --release --bin kotadb -- -d ./data/analysis search-code "async fn"
cargo run --release -- benchmark --operations 1000  # Compare against benchmarks
```

### Future Testing Methods (Once Available)
```bash
# MCP server dogfooding (coming soon)
cargo run --bin mcp_server --features="mcp-server" --config kotadb-mcp-dev.toml &
# Then connect Claude Code to test:
# - Document search via MCP
# - Symbol navigation
# - Code intelligence queries
# - Performance under AI assistant load

# API dogfooding (future)
cargo run --release -- server --port 8080 &
# Test via HTTP API for integration scenarios
```

### Dogfooding Protocol - FOLLOW THIS RELIGIOUSLY

#### 1. Before Starting Any Work
```bash
# Always start with fresh dogfooding setup
rm -rf data/analysis
mkdir -p data/analysis
cargo run --bin kotadb -- -d ./data/analysis index-codebase .
cargo run --bin kotadb -- -d ./data/analysis stats --symbols
```

#### 2. During Development
**Test your changes continuously against the live codebase:**
- After every significant change, re-index and test critical queries
- Use the exact same queries AI assistants would use
- Test performance with realistic data loads
- Verify symbol extraction accuracy on actual complex code

#### 3. Before Submitting PRs
**Mandatory dogfooding validation:**
- Full re-index of KotaDB codebase
- Test all major query types (content, symbols, relationships, impact)
- Performance regression testing
- Edge case validation (empty results, malformed queries, large datasets)
- Create GitHub issues for any problems found

### Why Dogfood? (Proven Track Record)
Real-world testing on KotaDB's own codebase consistently reveals integration issues that unit tests miss:
- **Issue #191**: Search disconnection after git ingestion (found only through dogfooding)
- **Issue #196**: Trigram index architectural limitation (discovered during self-analysis) 
- **Issue #184**: Comprehensive validation revealed multiple UX and functionality gaps
- **Issue #179**: Symbol extraction edge cases only surfaced with real Rust code complexity
- **Issue #203**: Performance degradation under realistic query patterns
- **Issue #157**: Memory usage issues only visible with large codebases

**Pattern**: Every major integration bug has been caught by dogfooding, not unit tests.

### Dogfooding Best Practices - STRICTLY ENFORCE

#### Directory Structure
```bash
# ALWAYS use separate analysis directory
data/analysis/           # Dogfooding tests - DELETE after use
data/test-scenarios/     # Specific test cases - DELETE after use  
kota-db-data/           # Normal usage data - OK to keep
```

#### Testing Scenarios
1. **Fresh Repository Analysis**
   - Clone KotaDB to temp directory
   - Index from scratch
   - Validate symbol extraction completeness

2. **Incremental Updates**
   - Make code changes
   - Test incremental indexing
   - Verify consistency

3. **Performance Under Load**
   - Multiple concurrent queries
   - Large result sets
   - Complex relationship traversals

4. **Integration Points**
   - MCP server responsiveness
   - API endpoint behavior
   - CLI tool usability

#### Issue Creation Protocol
**When you find problems through dogfooding:**
```bash
# IMMEDIATELY create GitHub issue
gh issue create \
  --title "[Dogfooding] Found issue: [brief description]" \
  --body "
## Problem Found During Dogfooding
**Test scenario:** [what you were testing]
**Expected behavior:** [what should happen]
**Actual behavior:** [what actually happened]
**Reproduction steps:** [exact commands to reproduce]
**Impact:** [how this affects real usage]
**Data:** [relevant stats, performance numbers, error messages]
" \
  --label "bug,dogfooding,priority-high"
```

### Quality Gate
**NO CODE SHIPS WITHOUT DOGFOODING VALIDATION**
- All PRs must include dogfooding test results in description
- Performance regression tests must pass
- Symbol extraction must be >95% accurate on KotaDB codebase
- Query latency must remain <10ms for typical operations
- All integration points must work smoothly

This is how we maintain KotaDB's 99% reliability while shipping AI-first features.

## 🏛️ Architecture Principles

### 1. Risk Reduction First
The entire codebase is built on a **6-stage risk reduction methodology**:
1. **Test-Driven Development** (-5.0 risk) - Tests written before implementation
2. **Contract-First Design** (-5.0 risk) - Formal traits with pre/post conditions
3. **Pure Function Modularization** (-3.5 risk) - Business logic in pure functions
4. **Comprehensive Observability** (-4.5 risk) - Tracing, metrics, structured logging
5. **Adversarial Testing** (-0.5 risk) - Property-based and chaos testing
6. **Component Library** (-1.0 risk) - Validated types, builders, wrappers

**Total Risk Reduction**: -19.5 points (99% success rate)

### 2. Component Library Pattern
ALWAYS use the component library:
```rust
// ✅ CORRECT - Use the factory function with all wrappers
let storage = create_file_storage("data", Some(1000)).await?;

// ❌ WRONG - Direct instantiation bypasses safety
let storage = FileStorage::new("data").await?;

// ✅ CORRECT - Use builder patterns
let doc = DocumentBuilder::new()
    .path("/test.md")?
    .title("Test Document")?
    .content(b"content")?
    .build()?;

// ✅ CORRECT - Use validated types
let path = ValidatedPath::new("/valid/path.md")?; // Compile-time safety
```

### 3. Never Break Safety Guarantees
- **NEVER** use `.unwrap()` in production code
- **ALWAYS** use the validation layer for user inputs
- **ALWAYS** use the observability wrappers for tracing
- **ALWAYS** handle errors properly with `anyhow::Result`

## 📁 Critical File Structure

```
kota-db/
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── main.rs             # CLI binary entry point
│   ├── types.rs            # Core data structures
│   ├── validation.rs       # Input validation layer
│   ├── contracts/          # Trait definitions with contracts
│   ├── wrappers/           # Stage 6 safety wrappers
│   ├── pure/               # Pure functions (business logic)
│   ├── file_storage.rs     # ✅ COMPLETE - File-based storage
│   ├── primary_index.rs    # 🔄 IN PROGRESS - B+ tree index
│   └── trigram_index.rs    # 🔄 IN PROGRESS - Full-text search
│
├── tests/
│   ├── integration_tests/  # End-to-end tests
│   ├── property_tests/     # Property-based tests
│   └── performance_tests/  # Performance regression tests
│
├── .github/
│   ├── workflows/ci.yml    # CI/CD pipeline (DO NOT BREAK)
│   └── ISSUE_TEMPLATE/     # Use these for creating issues
│
├── justfile               # Development commands (USE THIS)
├── Cargo.toml            # Dependencies and project config
└── run_standalone.sh     # Alternative to justfile
```

## 🧪 Testing Standards & Requirements

### ⚠️ **CRITICAL: Anti-Mock Testing Philosophy**

**❌ NEVER USE MOCKS OR STUBS**
This project follows a **strict anti-mock policy**. LLMs love to mock things, but we use **real implementations with failure injection** instead.

**✅ USE THESE PATTERNS INSTEAD:**
- **Failure Injection**: `FlakyStorage`, `DiskFullStorage`, `SlowStorage`
- **Temporary Directories**: `TempDir::new()` for isolated test environments
- **Real Components**: Always use actual storage/index implementations
- **Builder Patterns**: `create_test_storage()`, `create_test_document()`

### Test Coverage Requirements
- **Unit tests**: >90% coverage (243 tests currently passing)
- **Integration tests**: All major workflows
- **Property tests**: All core algorithms using `proptest`
- **Performance tests**: Sub-10ms latency validated
- **Adversarial tests**: Chaos engineering with real failure scenarios

### Before Every Commit
```bash
# MANDATORY quality gates
just fmt-check     # Code formatting
just clippy        # Linting (must pass with -D warnings)
just test-unit     # Unit tests
just test-integration  # Integration tests
just audit         # Security audit

# Or run all at once
just ci
```

### Test Patterns to Follow
```rust
// ✅ Use the test helpers from the component library
#[tokio::test]
async fn test_storage_operations() -> Result<()> {
    let storage = create_test_storage().await?;  // Real storage in temp dir
    
    let doc = create_test_document()?;           // Builder pattern
    storage.insert(doc.clone()).await?;
    
    let retrieved = storage.get(&doc.id).await?;
    assert_eq!(retrieved.unwrap().content, doc.content);
    Ok(())
}

// ✅ Use property-based testing for algorithms
proptest! {
    #[test]
    fn trigram_generation_is_consistent(s in ".*") {
        let trigrams1 = generate_trigrams(&s);
        let trigrams2 = generate_trigrams(&s);
        prop_assert_eq!(trigrams1, trigrams2);
    }
}

// ✅ Use failure injection instead of mocks
#[tokio::test]
async fn test_storage_failure_handling() -> Result<()> {
    let storage = FlakyStorage::new(0.5).await?; // 50% failure rate
    // Test with real storage that randomly fails
    let result = storage.insert(doc).await;
    // Verify error handling works correctly
    Ok(())
}
```

### Test Organization (22 Test Suites)
```
tests/
├── adversarial_tests.rs      # Chaos engineering with failure injection
├── bulk_operations_test.rs   # Performance and throughput testing
├── chaos_tests.rs           # System resilience testing
├── property_tests/          # Property-based algorithm testing
└── ...                      # 18 more comprehensive test suites
```

## 🚀 CI/CD Pipeline

### Automated Checks (DO NOT BREAK)
Every PR triggers:
1. **Formatting** - `cargo fmt --check`
2. **Linting** - `cargo clippy -- -D warnings`
3. **Tests** - Unit, integration, doc tests
4. **Security** - `cargo audit`
5. **Coverage** - Uploaded to Codecov
6. **Performance** - Regression tests
7. **Documentation** - Must build successfully
8. **Container** - Docker build validation

### Release Process & Versioning

KotaDB follows **Semantic Versioning** (MAJOR.MINOR.PATCH) with comprehensive release automation.

#### 🚨 CRITICAL: Release from develop branch
**ALWAYS perform releases from the `develop` branch, never from `main`:**
```bash
# 1. Ensure you're on develop and up to date
git checkout develop
git pull origin develop

# 2. Run the release command
just release-patch  # or release-minor, release-major

# 3. After release completes, the changes will be on main
# 4. Merge main back to develop to sync version numbers
git fetch origin main
git merge origin/main -m "chore: merge version updates from release"
git push origin develop
```

#### Quick Release Commands
```bash
# Check current version
just version                 # Shows current version from Cargo.toml

# Preview what's in next release
just release-preview         # Shows unreleased changes and recent commits

# Automatic version bump releases (RUN FROM DEVELOP)
just release-patch           # 0.1.0 -> 0.1.1 (bug fixes)
just release-minor           # 0.1.0 -> 0.2.0 (new features)
just release-major           # 0.1.0 -> 1.0.0 (breaking changes)
just release-beta            # 0.1.0 -> 0.1.0-beta.1 (prerelease)

# Release specific version (RUN FROM DEVELOP)
just release 0.2.0           # Full automated release process

# Test the release process
just release-dry-run 0.2.0   # Dry run without making changes
```

#### Release Process Details
The automated release (`scripts/release.sh`) will:
1. ✅ Verify clean working directory
2. ✅ Run all tests and quality checks
3. ✅ Update version in Cargo.toml, VERSION file, CHANGELOG.md
4. ✅ Update client library versions (Python, TypeScript, Go)
5. ✅ Commit all changes with proper message
6. ✅ Create annotated git tag with changelog excerpt
7. ✅ Push to remote (with confirmation prompt)

#### GitHub Actions Automation
Once a tag is pushed, GitHub Actions automatically:
- 📦 Creates GitHub Release with changelog notes
- 🔨 Builds binaries for all platforms (Linux, macOS, Windows)
- 🐳 Publishes Docker images to ghcr.io
- 📚 Publishes to crates.io (non-prerelease only)

#### Version Files
- `Cargo.toml` - Main version source
- `VERSION` - Plain text version file
- `CHANGELOG.md` - Version history with changes
- `docs/RELEASE_PROCESS.md` - Complete release guide

## 🔍 Debugging & Observability

### Logging Setup
```bash
# Enable comprehensive logging
export RUST_LOG=debug
export RUST_BACKTRACE=full

# Module-specific logging
export RUST_LOG=kotadb::storage=debug,kotadb::index=info

# Run with tracing
just dev  # Automatically includes trace IDs
```

### Performance Monitoring
```bash
# Monitor key metrics
just bench              # Run benchmarks
just test-perf          # Performance regression tests
just profile kotadb     # CPU profiling

# Check performance targets
# - Query latency: <10ms
# - Bulk operations: 10x speedup
# - Memory overhead: <2.5x raw data
```

## 🐳 Container Development

### Development Environment
```bash
# Full development environment with all services
just docker-up       # Starts kotadb-dev, docs-server, redis-dev, postgres-dev
just docker-shell    # Connect to main development container

# Available services:
# - kotadb-dev: Main development (port 8080)
# - docs-server: Documentation (port 8001)
# - redis-dev: Development cache (port 6379)
# - postgres-dev: Test database (port 5432)
```

## 🔒 Security & Safety

### Security Requirements
- **NEVER** commit secrets or API keys
- **ALWAYS** use `cargo audit` before releases
- **ALWAYS** handle user input through validation layer
- **NEVER** use unsafe code without extensive justification

### Memory Safety
- Use Rust's ownership system properly
- Prefer `Arc<T>` over `Rc<T>` for threaded code
- Use `tokio::sync` primitives for async coordination

## 📚 Knowledge Sources

### Primary Documentation
1. **This file (AGENT.md)** - Essential agent guide
2. **AGENT_CONTEXT.md** - Project context and status
3. **DEV_GUIDE.md** - Detailed development workflow
4. **README.md** - Project overview and features

### Code Understanding
1. **src/lib.rs** - Library entry point and public API
2. **src/contracts/** - Trait definitions and contracts
3. **src/wrappers/** - Stage 6 component library
4. **docs/** directory - Architecture and design docs

### GitHub Integration
- **Issues** - Current work and priorities
- **PRs** - Code reviews and discussions
- **Wiki** - Additional documentation
- **Discussions** - Architecture decisions

## ⚡ Performance Targets

### Latency Requirements
- Document retrieval: <1ms
- Text search queries: <10ms
- Graph traversals: <50ms
- Semantic search: <100ms

### Throughput Requirements
- Document inserts: >1,000/sec
- Bulk operations: >10,000/sec
- Concurrent queries: >100/sec

### Resource Limits
- Memory overhead: <2.5x raw data size
- Disk space: <1.5x raw data size
- CPU usage: <50% during normal operations

## 🎯 Code Style & Conventions

### Rust Conventions
```rust
// ✅ Use descriptive names
fn validate_document_path(path: &str) -> Result<ValidatedPath> { }

// ✅ Use builder patterns for complex objects
DocumentBuilder::new()
    .path("/path/to/doc.md")?
    .title("My Document")?
    .build()?

// ✅ Use the type system for safety
struct ValidatedPath(String);  // Cannot be constructed invalidly

// ✅ Comprehensive error handling
#[derive(thiserror::Error, Debug)]
enum StorageError {
    #[error("Document not found: {id}")]
    DocumentNotFound { id: DocumentId },
}
```

### Commit Message Format
```bash
# Format: type(scope): description
feat(mcp): add semantic search tool
fix(storage): resolve memory leak in bulk operations
docs(api): add examples for document builder
test(index): add property tests for B+ tree
perf(query): optimize graph traversal algorithm
refactor(types): simplify validation layer
chore: update dependencies
ci: add new test workflow
```

### Changelog Maintenance
**IMPORTANT**: Always update CHANGELOG.md for user-facing changes:

```markdown
## [Unreleased]

### Added
- New feature or capability

### Changed
- Changes to existing functionality

### Fixed
- Bug fixes

### Deprecated
- Features that will be removed

### Removed
- Features that were removed

### Security
- Security vulnerability fixes
```

Run `just changelog-update` to add a new Unreleased section after a release.

## 🚨 Common Pitfalls to Avoid

### ❌ DO NOT
- Use `.unwrap()` or `.expect()` in production code
- Bypass the validation layer for user inputs
- Skip writing tests for new functionality
- Break the existing CI/CD pipeline
- Commit without running `just check`
- Work without commenting on GitHub issues
- Add dependencies without careful consideration

### ✅ DO
- Use the component library patterns
- Follow the 6-stage risk reduction methodology
- Comment extensively on GitHub issues and PRs
- Run `just check` before every commit
- Use builder patterns for complex construction
- Handle all errors properly with `Result<T>`
- Write comprehensive tests for new features

## 📞 Getting Help

### When Stuck
1. **Check GitHub issues** - Someone may have faced this before
2. **Read the docs/** directory - Comprehensive architecture docs
3. **Run the demo** - `just demo` shows working patterns
4. **Check recent PRs** - See what other agents have done
5. **Create an issue** - Document the problem for future agents

### Emergency Procedures
If you break something critical:
1. **Immediately comment on the relevant issue**
2. **Create a new issue** with details of what broke
3. **Revert the breaking change** if possible
4. **Run `just ci`** to verify the fix
5. **Document the learning** for future agents

---

## 🎓 Final Notes for Agents

Remember: **You are part of a team of LLM agents working together through GitHub.** Your code will be reviewed, modified, and extended by other agents. Write code and documentation as if you're teaching the next agent how to continue your work.

**The project's success depends on maintaining the 99% reliability achieved through the 6-stage risk reduction methodology. Never compromise safety for speed.**

Every line of code you write should make the system more reliable, more maintainable, and more understandable for the next agent who works on it.

Good luck! 🤖✨
