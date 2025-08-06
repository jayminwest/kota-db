# Contributing to KotaDB

Thank you for your interest in contributing to KotaDB! This document provides guidelines and instructions for contributing to the project.

## Development Setup

### Prerequisites
- Rust 1.70+ (managed via `rust-toolchain.toml`)
- Git
- Basic understanding of database concepts and Rust

### Getting Started

1. **Fork and Clone**
   ```bash
   git clone https://github.com/jayminwest/kota-db.git
   cd kota-db
   ```

2. **Install Dependencies**
   ```bash
   # Rust toolchain is automatically installed via rust-toolchain.toml
   cargo build
   ```

3. **Run Tests**
   ```bash
   ./run_standalone.sh test
   ```

4. **Run Demo**
   ```bash
   ./run_standalone.sh demo
   ```

## Project Structure

```
kota-db/
├── src/                    # Core implementation
│   ├── contracts/          # Trait definitions and interfaces
│   ├── types.rs           # Validated types and data structures
│   ├── builders.rs        # Builder patterns for ergonomic APIs
│   ├── wrappers.rs        # Component wrappers (caching, tracing, etc.)
│   ├── file_storage.rs    # File-based storage implementation
│   ├── primary_index.rs   # B+ tree primary index
│   ├── trigram_index.rs   # Full-text search index
│   ├── pure/              # Pure functions (no side effects)
│   ├── metrics/           # Performance and optimization metrics
│   └── observability.rs   # Logging, tracing, and monitoring
├── tests/                 # Integration and unit tests
├── docs/                  # Comprehensive documentation
├── examples/              # Usage examples and demos
├── benches/               # Performance benchmarks
└── handoffs/              # Development history and handoff docs
```

## Architecture Principles

KotaDB follows a **6-stage risk reduction methodology**:

1. **Test-Driven Development** - Tests written before implementation
2. **Contract-First Design** - Clear interfaces and preconditions
3. **Pure Function Modularization** - Business logic separated from I/O
4. **Comprehensive Observability** - Full tracing and metrics
5. **Adversarial Testing** - Edge cases and failure scenarios
6. **Component Library** - Validated types and automatic best practices

### Key Design Patterns

- **Validated Types**: Invalid states are unrepresentable
- **Builder Patterns**: Fluent APIs with sensible defaults
- **Wrapper Components**: Automatic cross-cutting concerns (caching, tracing, retries)
- **Pure Functions**: Predictable, testable business logic
- **Contract Validation**: Runtime precondition and postcondition checking

## Development Workflow

### 1. Issue Creation
Before starting work, create an issue describing:
- Problem statement or feature request
- Proposed solution approach
- Expected impact and risks
- Testing strategy

### 2. Branch Strategy
```bash
# Create feature branch
git checkout -b feature/your-feature-name

# Create bugfix branch  
git checkout -b fix/issue-description

# Create MCP integration branch
git checkout -b mcp/component-name
```

### 3. Implementation Guidelines

#### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add documentation for public APIs
- Include examples in documentation

#### Testing Requirements
- Unit tests for all public functions
- Integration tests for workflows
- Property-based tests for algorithms
- Performance regression tests

#### Error Handling
- Use `anyhow::Result` for general errors
- Create specific error types for domain errors
- Never use `unwrap()` in production code
- Provide helpful error messages

#### Performance
- Benchmark performance-critical code
- Use the metrics infrastructure for monitoring
- Follow the Stage 6 component patterns
- Cache frequently accessed data

### 4. Commit Guidelines

Follow conventional commits format:
```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `test`: Test additions or modifications
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `ci`: CI/CD changes

Examples:
```bash
feat(mcp): add semantic search tool for LLM integration
fix(storage): resolve memory leak in bulk operations
docs(api): add examples for document builder patterns
test(index): add property tests for B+ tree operations
```

### 5. Pull Request Process

1. **Pre-PR Checklist**
   - [ ] All tests pass (`cargo test`)
   - [ ] Code is formatted (`cargo fmt`)
   - [ ] No clippy warnings (`cargo clippy`)
   - [ ] Documentation updated
   - [ ] Examples added if needed
   - [ ] Performance impact assessed

2. **PR Description Template**
   ```markdown
   ## Summary
   Brief description of changes
   
   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update
   
   ## Testing
   - [ ] Unit tests added/updated
   - [ ] Integration tests added/updated
   - [ ] Manual testing performed
   
   ## Performance Impact
   - [ ] No performance impact
   - [ ] Performance improvement
   - [ ] Performance regression (justified)
   
   ## Checklist
   - [ ] Code follows project style guidelines
   - [ ] Self-review completed
   - [ ] Documentation updated
   - [ ] Tests added for new functionality
   ```

3. **Review Process**
   - All PRs require at least one approval
   - CI must pass before merging
   - Address all review feedback
   - Squash commits when merging

## Specific Contribution Areas

### MCP Server Development
Priority area for LLM integration:
- JSON-RPC protocol implementation
- Semantic search tools
- Document management APIs
- Performance optimization

See `MCP_INTEGRATION_PLAN.md` for detailed specifications.

### Test Coverage Improvement
Help complete the test suite:
- Implement missing `todo!()` functions in test files
- Add property-based tests for algorithms
- Create end-to-end integration scenarios

### Performance Optimization
Enhance database performance:
- Optimize B+ tree operations
- Improve cache hit rates
- Reduce memory allocations
- Add performance benchmarks

### Documentation
Improve project documentation:
- API documentation with examples
- Tutorial and getting started guides
- Architecture decision records
- Performance tuning guides

## Quality Standards

### Code Quality
- 100% of public APIs must be documented
- All `unwrap()` calls must be justified or replaced
- Error handling must be comprehensive
- Performance regressions are not acceptable

### Testing Standards
- Unit test coverage >90%
- All edge cases must be tested
- Performance tests for critical paths
- Property-based testing for algorithms

### Documentation Standards
- All public APIs have rustdoc comments
- Examples included for complex APIs
- Architecture decisions documented
- Performance characteristics documented

## Getting Help

### Communication Channels
- GitHub Issues for bugs and feature requests
- GitHub Discussions for questions and ideas
- Code review comments for implementation feedback

### Resources
- `docs/` directory for comprehensive documentation
- `examples/` directory for usage patterns
- `AGENT_CONTEXT.md` for project overview
- `OUTSTANDING_ISSUES.md` for current priorities

### Mentorship
New contributors are welcome! We provide:
- Code review feedback and guidance
- Architecture discussions
- Performance optimization tips
- Testing strategy assistance

## Recognition

Contributors are recognized through:
- GitHub contributor statistics
- Mention in release notes
- Credit in documentation
- Community appreciation

## License

By contributing to KotaDB, you agree that your contributions will be licensed under the same license as the project (currently proprietary, shared for educational purposes).

---

Thank you for contributing to KotaDB! Your efforts help make this database the best solution for LLM-powered knowledge management.
