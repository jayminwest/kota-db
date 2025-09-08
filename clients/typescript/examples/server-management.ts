/**
 * KotaDB Server Management Examples
 * 
 * This example demonstrates how to automatically download, install, and manage
 * a KotaDB server instance using the TypeScript client library.
 */

import { KotaDB, KotaDBServer, startServer, ensureBinaryInstalled } from '../src';
import * as path from 'path';
import * as os from 'os';

/**
 * Example 1: Basic server management with async/await
 */
async function example1BasicUsage() {
  console.log('='.repeat(60));
  console.log('Example 1: Basic Server Management');
  console.log('='.repeat(60));
  
  // Create and start server
  const server = new KotaDBServer({
    port: 8080,
    dataDir: path.join(os.tmpdir(), 'kotadb-example1')
  });
  
  try {
    await server.start();
    console.log('Server is running...');
    
    // Connect client to the server
    const client = new KotaDB({ url: 'http://localhost:8080' });
    
    // Perform some operations
    const docId = await client.insert({
      path: '/example/doc1.md',
      title: 'Test Document',
      content: 'This document was created with auto-managed server',
      tags: ['example', 'auto-server']
    });
    console.log(`Created document: ${docId}`);
    
    // Search for the document
    const results = await client.search('auto-managed');
    console.log(`Found ${results.length} results`);
    
  } finally {
    // Always stop the server
    server.stop();
    console.log('Server stopped');
  }
}

/**
 * Example 2: Using the convenience function
 */
async function example2ConvenienceFunction() {
  console.log('\n' + '='.repeat(60));
  console.log('Example 2: Convenience Function');
  console.log('='.repeat(60));
  
  // Quick start with convenience function
  const server = await startServer({
    port: 8081,
    dataDir: path.join(os.tmpdir(), 'kotadb-example2')
  });
  
  try {
    console.log('Server started with convenience function');
    
    // Use the server
    const client = new KotaDB({ url: 'http://localhost:8081' });
    
    // Batch insert example
    const docIds: string[] = [];
    for (let i = 0; i < 5; i++) {
      const docId = await client.insert({
        path: `/convenience/doc${i}.md`,
        title: `Document ${i}`,
        content: `Content for document ${i}`,
        tags: ['batch', 'example']
      });
      docIds.push(docId);
    }
    
    console.log(`Created ${docIds.length} documents`);
    
    // List all documents
    const docs = await client.list();
    console.log(`Total documents in database: ${docs.length}`);
    
  } finally {
    server.stop();
  }
}

/**
 * Example 3: Binary installation without starting server
 */
async function example3BinaryInstallation() {
  console.log('\n' + '='.repeat(60));
  console.log('Example 3: Binary Installation Only');
  console.log('='.repeat(60));
  
  try {
    // Ensure binary is installed (useful for CI/CD or containers)
    const binaryPath = await ensureBinaryInstalled();
    console.log(`KotaDB binary installed at: ${binaryPath}`);
    
    // Verify binary exists
    const fs = require('fs');
    const stats = fs.statSync(binaryPath);
    console.log('✓ Binary verification successful');
    console.log(`  Size: ${stats.size.toLocaleString()} bytes`);
    console.log(`  Executable: ${(stats.mode & 0o111) !== 0}`);
  } catch (error) {
    console.error('Failed to install binary:', error);
  }
}

/**
 * Example 4: Multiple server instances
 */
async function example4MultipleServers() {
  console.log('\n' + '='.repeat(60));
  console.log('Example 4: Multiple Server Instances');
  console.log('='.repeat(60));
  
  const servers: KotaDBServer[] = [];
  
  try {
    // Development server
    const devServer = new KotaDBServer({
      port: 8090,
      dataDir: path.join(os.tmpdir(), 'kotadb-dev')
    });
    await devServer.start();
    servers.push(devServer);
    console.log('Development server started on port 8090');
    
    // Test server
    const testServer = new KotaDBServer({
      port: 8091,
      dataDir: path.join(os.tmpdir(), 'kotadb-test')
    });
    await testServer.start();
    servers.push(testServer);
    console.log('Test server started on port 8091');
    
    // Connect to both
    const devClient = new KotaDB({ url: 'http://localhost:8090' });
    const testClient = new KotaDB({ url: 'http://localhost:8091' });
    
    // Add data to each
    await devClient.insert({
      path: '/dev/config.md',
      title: 'Development Config',
      content: 'Development environment configuration'
    });
    
    await testClient.insert({
      path: '/test/suite.md',
      title: 'Test Suite',
      content: 'Test suite documentation'
    });
    
    console.log('✓ Both servers operational');
    
  } finally {
    // Clean up all servers
    servers.forEach(server => server.stop());
    console.log('All servers stopped');
  }
}

/**
 * Example 5: Error handling and recovery
 */
async function example5ErrorHandling() {
  console.log('\n' + '='.repeat(60));
  console.log('Example 5: Error Handling');
  console.log('='.repeat(60));
  
  const server = new KotaDBServer({
    port: 8092,
    autoInstall: true  // Will auto-download if needed
  });
  
  try {
    // Start server with timeout
    await server.start(undefined, 15000);  // 15 second timeout
    console.log('Server started successfully');
    
    // Check if running
    const isRunning = await server.isRunning();
    console.log(`Server status: ${isRunning ? 'Running' : 'Not running'}`);
    
    // Connect and test
    const client = new KotaDB({ 
      url: 'http://localhost:8092',
      timeout: 5000  // 5 second timeout for operations
    });
    
    try {
      await client.insert({
        path: '/test/error-handling.md',
        title: 'Error Handling Test',
        content: 'Testing error handling capabilities'
      });
      console.log('✓ Server operations working');
    } catch (error) {
      console.error('Operation failed:', error);
    }
    
  } catch (error) {
    console.error('Failed to start server:', error);
    console.log('This might happen if:');
    console.log('- Port is already in use');
    console.log('- Binary download failed');
    console.log('- Insufficient permissions');
  } finally {
    server.stop();
  }
}

/**
 * Main function to run all examples
 */
async function main() {
  console.log('\n🚀 KotaDB Server Management Examples\n');
  
  try {
    await example1BasicUsage();
    await example2ConvenienceFunction();
    await example3BinaryInstallation();
    await example4MultipleServers();
    await example5ErrorHandling();
    
    console.log('\n' + '='.repeat(60));
    console.log('✅ All examples completed successfully!');
    console.log('='.repeat(60));
  } catch (error) {
    console.error('\n❌ Error:', error);
    process.exit(1);
  }
}

// Run examples if executed directly
if (require.main === module) {
  main().catch(console.error);
}

export { main };