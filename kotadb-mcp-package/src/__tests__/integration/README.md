# MCP Package Integration Testing

This directory contains comprehensive integration tests for the KotaDB MCP package, implementing the testing requirements outlined in issue #124.

## Overview

The integration tests validate real-world MCP usage scenarios with Claude Desktop and other MCP clients, ensuring protocol compliance and end-to-end functionality.

## Test Architecture

### Anti-Mock Philosophy

Following KotaDB's anti-mock testing philosophy, these tests use **real implementations with failure injection** instead of mocks or stubs:

- ✅ **Real MCP server processes** spawned for testing  
- ✅ **Real JSON-RPC communication** over stdio  
- ✅ **Real file system operations** in temporary directories  
- ✅ **Failure injection helpers** for stress testing  
- ❌ **No mocks or stubs** - everything uses actual implementations  

### Test Structure

```
integration/
├── README.md                     # This file
├── test-helpers.ts              # Comprehensive test utilities
├── protocol-compliance.test.ts  # JSON-RPC and MCP protocol tests
├── real-world-scenarios.test.ts # End-to-end workflow tests
├── cross-platform.test.ts       # Platform compatibility tests
└── stress-performance.test.ts    # Load and performance tests
```

## Test Categories

### 1. Protocol Compliance (`protocol-compliance.test.ts`)

Validates JSON-RPC 2.0 and MCP protocol compliance:

- **JSON-RPC Protocol**: Request/response formats, error handling
- **MCP Initialization**: Capabilities discovery, handshake validation
- **Tool Execution**: Schema validation, structured responses
- **Resource Management**: URI handling, content reading
- **Error Handling**: Graceful failure modes, server stability

### 2. Real-World Scenarios (`real-world-scenarios.test.ts`)

Tests complete user workflows:

- **New User Onboarding**: First-time setup and document creation
- **Knowledge Management**: Document CRUD operations, search workflows
- **Collaborative Workflows**: Multiple concurrent users simulation
- **Performance Under Load**: Growing document collections
- **Data Persistence**: Cross-session data integrity

### 3. Cross-Platform Compatibility (`cross-platform.test.ts`)

Ensures functionality across different environments:

- **Platform Detection**: macOS, Linux, Windows compatibility
- **File System Handling**: Path formats, Unicode support, case sensitivity
- **Environment Variables**: Configuration and data directory handling
- **Binary Distribution**: Package installation and binary location
- **Memory Management**: Resource usage across platforms

### 4. Stress and Performance (`stress-performance.test.ts`)

Validates system behavior under stress:

- **High Volume Operations**: Batch document creation, concurrent searches
- **Memory Management**: Resource cleanup, leak detection
- **Network Simulation**: Backpressure, timeout handling, failure recovery
- **Long-Running Operations**: Performance stability over time
- **Resource Exhaustion**: Recovery from system pressure

## Test Utilities (`test-helpers.ts`)

### MCPTestClient Class

Robust test client that manages MCP server lifecycle:

```typescript
const client = await createTestClient();

// Document operations
const doc = await client.createDocument({
  path: '/test.md',
  content: 'Test content'
});

// Search operations
const results = await client.searchDocuments('test');

// Cleanup
await client.cleanup();
```

### Performance Monitoring

```typescript
const timer = new PerformanceTimer();
timer.start();
await someOperation();
const duration = timer.end();
console.log(`Operation took ${duration}ms`);
```

### Error Injection

```typescript
const flakyClient = new ErrorInjectionClient(0.3, 50); // 30% failure, 50ms delay
// Test with simulated network issues
```

## Running Tests

### Individual Test Suites

```bash
# Run all integration tests
npm run test:integration

# Run specific test suites
npm run test:integration -- --testNamePattern="Protocol Compliance"
npm run test:integration -- --testNamePattern="Real-World Scenarios"
npm run test:integration -- --testNamePattern="Cross-Platform"
npm run test:integration -- --testNamePattern="Stress Testing"
```

### Combined Testing

```bash
# Run unit and integration tests
npm run test:all

# Run with coverage
npm run test:coverage

# Watch mode for development
npm run test:watch
```

## Test Configuration

### Environment Variables

- `KOTADB_BINARY_PATH`: Override KotaDB binary location
- `KOTADB_DATA_DIR`: Override data directory (auto-generated for tests)
- `CI`: Enable CI-specific test configurations

### Jest Configuration

- **Timeout**: 30 seconds for integration tests
- **Concurrency**: Limited to 2 for stability
- **Setup/Teardown**: Global setup builds binaries if needed

## CI Integration

The tests are integrated into the main CI pipeline:

```yaml
# .github/workflows/ci.yml
- name: Run MCP integration tests
  working-directory: kotadb-mcp-package
  run: npm run test:integration
  env:
    KOTADB_BINARY_PATH: ../target/release/kotadb
```

## Test Design Principles

### 1. Real User Flows

Tests follow actual user workflows:

1. User discovers tools → Test tool listing
2. User creates document → Test document creation
3. User searches content → Test search functionality
4. User updates document → Test update operations

### 2. Comprehensive Coverage

- **All 7 MCP tools** are tested
- **Both success and error scenarios** are covered
- **Cross-platform compatibility** is validated
- **Performance benchmarks** are established

### 3. Deterministic Testing

- Tests use **temporary directories** for isolation
- **Cleanup is guaranteed** even on test failures
- **No external dependencies** that could cause flaky tests
- **Retry logic** for network-style operations

### 4. Performance Validation

- **Sub-10ms query latency** targets
- **Memory growth monitoring** 
- **Concurrent operation support**
- **Scalability with document count**

## Development Guidelines

### Adding New Tests

1. **Follow the anti-mock philosophy** - use real implementations
2. **Use test helpers** - leverage `MCPTestClient` and utilities
3. **Include cleanup** - ensure resources are released
4. **Test error scenarios** - not just happy path
5. **Document expected behavior** - clear test descriptions

### Test Structure

```typescript
describe('Feature Group', () => {
  let client: MCPTestClient;

  beforeAll(async () => {
    client = await createTestClient();
  });

  afterAll(async () => {
    await client.cleanup();
  });

  test('should handle specific scenario', async () => {
    // Arrange
    const testData = createTestDocument();
    
    // Act
    const result = await client.createDocument(testData);
    
    // Assert
    validateDocumentStructure(result);
    expect(result.content).toBe(testData.content);
  });
});
```

### Performance Testing

```typescript
test('should meet performance requirements', async () => {
  const timer = new PerformanceTimer();
  
  timer.start();
  const result = await client.performOperation();
  const duration = timer.end();
  
  expect(duration).toBeLessThan(100); // 100ms requirement
  expect(result).toBeDefined();
});
```

## Troubleshooting

### Common Issues

1. **Server startup timeout**: Increase timeout or check binary path
2. **Port conflicts**: Tests use stdio, not network ports
3. **Permission errors**: Verify temp directory access
4. **Binary not found**: Run `cargo build` in main project

### Debug Mode

```bash
# Run with verbose output
npm run test:integration -- --verbose

# Run with Jest debug info
npm run test:integration -- --detectOpenHandles

# Run single test file
npm run test:integration -- src/__tests__/integration/protocol-compliance.test.ts
```

## Metrics and Reporting

### Performance Benchmarks

- Document creation: < 300ms average
- Search operations: < 200ms average  
- Bulk operations: < 10x single operation time
- Memory overhead: < 100MB for test suites

### Coverage Requirements

- Integration test coverage: > 80% of MCP functionality
- Error path coverage: All major error scenarios
- Cross-platform validation: Core functionality on all platforms

## Future Enhancements

1. **Multi-client testing**: Simulate multiple Claude Desktop instances
2. **Network partition testing**: Test resilience to connection issues  
3. **Version compatibility**: Test MCP protocol version handling
4. **Resource monitoring**: Enhanced memory and CPU tracking
5. **Automated performance regression**: Detect performance degradation

This comprehensive integration testing suite ensures the KotaDB MCP package works reliably in real-world scenarios while maintaining the project's high standards for code quality and reliability.