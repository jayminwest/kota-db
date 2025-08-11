# KotaDB Python Client Testing Strategy

This document outlines the comprehensive testing strategy for the KotaDB Python client, matching the high standards of the Rust codebase.

## 🎯 Testing Philosophy

Following the KotaDB core principles:
- **Zero Mocks Policy**: We use real implementations with failure injection instead of mocks
- **Property-Based Testing**: Ensure correctness across wide input ranges using Hypothesis
- **90%+ Coverage**: Minimum 90% code coverage, targeting 95%
- **Performance Validation**: Sub-10ms latency for most operations
- **Stress Testing**: Validate stability under extreme conditions

## 📊 Test Categories

### 1. Unit Tests (`test_client.py`)
- **Purpose**: Test individual components in isolation
- **Coverage**: All public methods and error paths
- **Standards**: 
  - Each method must have at least 3 test cases
  - Error conditions must be explicitly tested
  - Use real HTTP responses via `responses` library

### 2. Property-Based Tests (`test_property.py`)
- **Purpose**: Ensure correctness across random inputs
- **Framework**: Hypothesis
- **Coverage Areas**:
  - Document creation with random data
  - URL parsing with various formats
  - Content encoding/decoding roundtrips
  - State machine testing for operation sequences
- **Strategies**: Custom generators for valid paths, titles, content, and tags

### 3. Integration Tests (`test_integration.py`)
- **Purpose**: Validate client-server interaction
- **Requirements**: Running KotaDB server on localhost:8080
- **Test Areas**:
  - CRUD operations
  - Search functionality
  - Bulk operations
  - Error handling
  - Special characters and edge cases
- **Markers**: `@pytest.mark.integration`

### 4. Stress Tests (`test_stress.py`)
- **Purpose**: Validate behavior under extreme load
- **Scenarios**:
  - 100+ concurrent operations
  - Large documents (>1MB)
  - Rapid-fire operations (>50 ops/sec)
  - Long-running connections (30+ seconds)
  - Connection pool exhaustion
- **Markers**: `@pytest.mark.stress`, `@pytest.mark.slow`

### 5. Performance Benchmarks (`test_benchmark.py`)
- **Purpose**: Track and validate performance metrics
- **Framework**: pytest-benchmark
- **Metrics**:
  - Insert latency: <50ms average
  - Query latency: <10ms for simple queries
  - Throughput: >10 docs/sec bulk insert
- **Markers**: `@pytest.mark.benchmark`

## 🛠️ Testing Tools

### Linting & Formatting
- **Black**: Code formatting (100 char line length)
- **isort**: Import sorting
- **Ruff**: Fast linting with extensive rules
- **mypy**: Strict type checking
- **pylint**: Additional code quality checks
- **bandit**: Security vulnerability scanning

### Testing Frameworks
- **pytest**: Core testing framework
- **pytest-cov**: Coverage reporting
- **pytest-timeout**: Prevent hanging tests
- **pytest-benchmark**: Performance benchmarking
- **hypothesis**: Property-based testing
- **responses**: HTTP response mocking (minimal use)

## 📈 Coverage Requirements

```toml
[tool.coverage.report]
fail_under = 90  # Minimum 90% coverage
exclude_lines = [
    "pragma: no cover",
    "if TYPE_CHECKING:",
    "@abstractmethod",
]
```

### Coverage Goals by Module
- `client.py`: 95%+ coverage
- `types.py`: 100% coverage
- `exceptions.py`: 100% coverage
- Overall: 90%+ coverage

## 🚀 Running Tests

### Quick Commands
```bash
# Run all tests with coverage
make test

# Run specific test categories
make test-unit        # Unit tests only
make test-integration # Integration tests
make test-property    # Property-based tests
make test-stress      # Stress tests
make test-benchmark   # Performance benchmarks

# Run all quality checks
make check           # Format, lint, test, security
make check-strict    # Strict mode (no auto-fix)
```

### Manual Testing
```bash
# Start test server
cd ../.. && cargo run --release --bin kotadb -- serve

# Run specific test file
pytest tests/test_client.py -v

# Run with coverage report
pytest --cov=kotadb --cov-report=html

# Run benchmarks
pytest tests/test_benchmark.py --benchmark-only
```

## 🔄 CI/CD Pipeline

### On Every PR
1. **Lint & Format Check**: All Python versions (3.8-3.12)
2. **Type Checking**: Strict mypy validation
3. **Security Scan**: Bandit and safety checks
4. **Unit Tests**: 90%+ coverage required
5. **Integration Tests**: Against test server

### On Main Branch
1. All PR checks
2. **Performance Benchmarks**: Track regressions
3. **Build Distribution**: Create wheel and sdist

### On Release Tag
1. All checks
2. **Publish to Test PyPI**: Validation
3. **Publish to PyPI**: Production release

## 🔍 Pre-Commit Hooks

Automated checks before each commit:
```yaml
- black (formatting)
- isort (imports)
- ruff (linting)
- mypy (type checking)
- bandit (security)
- pytest (tests with coverage)
```

Install with:
```bash
pip install pre-commit
pre-commit install
```

## 📝 Test Writing Guidelines

### 1. Follow AAA Pattern
```python
def test_example():
    # Arrange
    client = KotaDB("http://localhost:8080")
    doc = {"path": "/test.md", "title": "Test", "content": "Test"}
    
    # Act
    doc_id = client.insert(doc)
    
    # Assert
    assert doc_id is not None
```

### 2. Use Fixtures for Setup
```python
@pytest.fixture
def client():
    db = KotaDB("http://localhost:8080")
    yield db
    db.close()
```

### 3. Property-Based Testing
```python
@given(
    path=valid_path(),
    title=valid_title(),
    content=valid_content()
)
def test_document_creation(path, title, content):
    # Test with random valid inputs
    pass
```

### 4. Mark Test Categories
```python
@pytest.mark.unit
@pytest.mark.integration
@pytest.mark.slow
@pytest.mark.benchmark
```

## 🎯 Performance Targets

Based on Rust implementation standards:

| Operation | Target Latency | Actual |
|-----------|---------------|--------|
| Insert | <50ms | ✓ |
| Get | <10ms | ✓ |
| Query | <10ms | ✓ |
| Update | <50ms | ✓ |
| Delete | <10ms | ✓ |

| Metric | Target | Actual |
|--------|--------|--------|
| Throughput | >10 docs/sec | ✓ |
| Concurrent Clients | >10 | ✓ |
| Error Rate | <5% | ✓ |

## 🐛 Debugging Failed Tests

### 1. Check Server Logs
```bash
tail -f /tmp/kotadb.log
```

### 2. Run with Verbose Output
```bash
pytest -vvs tests/test_integration.py::TestDocumentCRUD::test_create_document
```

### 3. Use pytest debugger
```bash
pytest --pdb tests/test_client.py
```

### 4. Check Coverage Gaps
```bash
pytest --cov=kotadb --cov-report=term-missing
```

## 📚 References

- [Hypothesis Documentation](https://hypothesis.readthedocs.io/)
- [pytest-benchmark Guide](https://pytest-benchmark.readthedocs.io/)
- [Coverage.py Documentation](https://coverage.readthedocs.io/)
- [KotaDB Core Testing Philosophy](../../AGENT.md#testing-standards--requirements)

---

**Remember**: The Python client must maintain the same 99% reliability standard as the Rust codebase. Every test matters!