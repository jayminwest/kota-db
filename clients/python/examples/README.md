# KotaDB Python Client Examples

This directory contains comprehensive examples demonstrating the KotaDB Python client capabilities, including both traditional and new builder pattern approaches.

## Quick Start

Make sure you have a KotaDB server running before running these examples:

```bash
# Start KotaDB server (from project root)
cargo run --bin kotadb -- serve

# Install Python client
pip install kotadb-client

# Run any example
python examples/basic_usage.py
```

## Examples Overview

### 📝 [basic_usage.py](basic_usage.py)
**Traditional approach with core functionality**
- Basic CRUD operations (create, read, update, delete)
- Text and semantic search
- Error handling patterns
- Database health and statistics

```bash
python examples/basic_usage.py
```

### 🏗️ [builder_patterns.py](builder_patterns.py)
**Type safety and builder patterns demonstration**
- Validated types (`ValidatedPath`, `ValidatedDocumentId`, etc.)
- DocumentBuilder for safe document construction
- QueryBuilder for structured search queries
- UpdateBuilder for safe document updates
- Runtime validation and error prevention

```bash
python examples/builder_patterns.py
```

**Key features demonstrated:**
- Prevention of directory traversal attacks
- UUID validation for document IDs
- Title and content validation
- Fluent API with method chaining
- Type-safe construction patterns

### 🎯 [comprehensive_usage.py](comprehensive_usage.py)
**Side-by-side comparison of traditional vs builder approaches**
- Complete feature demonstration
- Traditional dictionary-based operations
- New builder pattern operations
- Advanced search patterns (text, semantic, hybrid)
- Error handling and validation
- Performance comparison

```bash
python examples/comprehensive_usage.py
```

**Best for:** Understanding the differences between approaches and when to use each.

### 🧪 [integration_test.py](integration_test.py)
**Integration testing against real KotaDB server**
- Comprehensive test suite for CI/CD
- CRUD operation testing
- Builder pattern validation
- Search capability testing
- Error handling verification
- Database information endpoints

```bash
# Run against default localhost:8080
python examples/integration_test.py

# Run against custom server
python examples/integration_test.py --url http://your-server:8080
```

**Features:**
- Automatic test data cleanup
- Detailed pass/fail reporting
- Configurable server URL
- Suitable for automated testing

### ⚡ [performance_test.py](performance_test.py)
**Performance benchmarking and measurement**
- Document insertion performance
- Document retrieval performance
- Search operation performance
- Builder pattern overhead analysis
- Bulk operations throughput
- Statistical analysis with timing

```bash
# Run default performance tests
python examples/performance_test.py

# Run with custom parameters
python examples/performance_test.py --insert-count 500 --search-count 100
```

**Performance targets:**
- Document insertion: <50ms average
- Document retrieval: <10ms average  
- Text search: <100ms average
- Builder overhead: <20% vs traditional
- Bulk operations: >100 docs/sec

### 🔗 [connection_examples.py](connection_examples.py)
**Various connection methods and configurations**
- Environment variable configuration
- Connection string formats
- Error handling and retries
- Connection pooling examples

```bash
python examples/connection_examples.py
```

## Usage Patterns

### Traditional Approach
Simple and direct, suitable for quick prototyping:

```python
from kotadb import KotaDB

db = KotaDB("http://localhost:8080")
doc_id = db.insert({
    "path": "/notes/meeting.md",
    "title": "Meeting Notes",
    "content": "Important meeting details...",
    "tags": ["work", "meeting"]
})
```

### Builder Pattern Approach
Type-safe with runtime validation, recommended for production:

```python
from kotadb import KotaDB, DocumentBuilder, ValidatedPath

db = KotaDB("http://localhost:8080")
doc_id = db.insert_with_builder(
    DocumentBuilder()
    .path(ValidatedPath("/notes/meeting.md"))
    .title("Meeting Notes") 
    .content("Important meeting details...")
    .add_tag("work")
    .add_tag("meeting")
)
```

## Type Safety Features

The Python client now provides runtime type safety equivalent to Rust's compile-time guarantees:

### Validated Types
- **ValidatedPath**: Prevents directory traversal, null bytes, reserved names
- **ValidatedDocumentId**: Ensures proper UUID format, prevents nil UUIDs
- **ValidatedTitle**: Enforces non-empty titles with length limits
- **ValidatedTimestamp**: Validates reasonable time ranges
- **NonZeroSize**: Ensures positive size values

### Builder Patterns
- **DocumentBuilder**: Safe document construction with validation
- **QueryBuilder**: Structured query building with filters
- **UpdateBuilder**: Safe document updates with operation tracking

### Error Prevention
- Runtime validation prevents common security issues
- Clear error messages for validation failures
- Type safety without compile-time overhead
- Consistent validation rules across all operations

## Testing Your Setup

Run the integration test to verify everything is working:

```bash
python examples/integration_test.py
```

This will test all major functionality and report any issues.

## Performance Benchmarking

Run performance tests to measure your setup:

```bash
python examples/performance_test.py
```

This will benchmark operations and compare against target performance metrics.

## Common Issues

### Connection Errors
```
❌ Failed to connect: Connection refused
```
**Solution:** Make sure KotaDB server is running on the specified port.

### Validation Errors
```
❌ ValidationError: Path contains null bytes
```
**Solution:** This is expected behavior - the validation is protecting against dangerous inputs.

### Import Errors
```
❌ ImportError: No module named 'kotadb'
```
**Solution:** Install the client: `pip install kotadb-client`

## Next Steps

1. **Start with basic_usage.py** to understand core concepts
2. **Try builder_patterns.py** to learn type safety features
3. **Run comprehensive_usage.py** to see all features together
4. **Use integration_test.py** for testing your setup
5. **Benchmark with performance_test.py** to measure performance

For more information, see the [main Python client documentation](../README.md).