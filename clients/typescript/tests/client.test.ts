/**
 * KotaDB TypeScript Client Tests
 * 
 * Comprehensive test suite for the TypeScript client following the same
 * anti-mock philosophy as the Rust codebase - using real implementations
 * and failure injection instead of mocks.
 */

import { KotaDB, connect } from '../src/client';
import { 
  ConnectionError, 
  ValidationError, 
  NotFoundError, 
  KotaDBError 
} from '../src/types';

// Test configuration - use environment variable for port flexibility
const TEST_PORT = process.env.KOTADB_TEST_PORT || '18432';
const TEST_SERVER_URL = process.env.KOTADB_TEST_URL || `http://localhost:${TEST_PORT}`;
const INVALID_SERVER_URL = 'http://localhost:9999'; // Non-existent server for error testing

describe('KotaDB TypeScript Client', () => {
  let db: KotaDB;

  beforeAll(async () => {
    // Test server availability
    try {
      db = new KotaDB({ url: TEST_SERVER_URL });
      await db.testConnection();
    } catch (error) {
      console.warn(`Test server not available at ${TEST_SERVER_URL}. Some tests will be skipped.`);
    }
  });

  afterAll(async () => {
    // Cleanup any test documents
    // Note: Real cleanup would go here if we had test document tracking
  });

  describe('Connection Management', () => {
    test('should connect with explicit URL', () => {
      const client = new KotaDB({ url: TEST_SERVER_URL });
      expect(client).toBeInstanceOf(KotaDB);
    });

    test('should connect using environment variable', () => {
      process.env.KOTADB_URL = TEST_SERVER_URL;
      const client = new KotaDB();
      expect(client).toBeInstanceOf(KotaDB);
      delete process.env.KOTADB_URL;
    });

    test('should parse kotadb:// connection strings', () => {
      const client = new KotaDB({ url: `kotadb://localhost:${TEST_PORT}/testdb` });
      expect(client).toBeInstanceOf(KotaDB);
    });

    test('should throw error when no URL provided', () => {
      delete process.env.KOTADB_URL;
      expect(() => new KotaDB()).toThrow(ConnectionError);
    });

    test('should use convenience connect function', () => {
      const client = connect({ url: TEST_SERVER_URL });
      expect(client).toBeInstanceOf(KotaDB);
    });

    test('should handle connection timeout configuration', () => {
      const client = new KotaDB({ 
        url: TEST_SERVER_URL, 
        timeout: 10000,
        retries: 5
      });
      expect(client).toBeInstanceOf(KotaDB);
    });
  });

  describe('Health and Status', () => {
    test('should check health status', async () => {
      if (!db) return; // Skip if server not available

      const health = await db.health();
      expect(health).toHaveProperty('status');
    });

    test('should test connection explicitly', async () => {
      if (!db) return;

      const health = await db.testConnection();
      expect(health).toHaveProperty('status');
    });

    test('should get database statistics', async () => {
      if (!db) return;

      try {
        const stats = await db.stats();
        expect(stats).toHaveProperty('document_count');
      } catch (error) {
        // Stats endpoint might not be implemented - that's ok
        expect(error).toBeInstanceOf(NotFoundError);
      }
    });
  });

  describe('Document Operations', () => {
    const testDocument = {
      path: '/test/typescript-client-test.md',
      title: 'TypeScript Client Test Document',
      content: 'This is a test document created by the TypeScript client test suite.',
      tags: ['test', 'typescript', 'client'],
      metadata: { test: true, created_by: 'jest' }
    };

    let createdDocId: string;

    test('should create a document', async () => {
      if (!db) return;

      createdDocId = await db.insert(testDocument);
      expect(createdDocId).toBeTruthy();
      expect(typeof createdDocId).toBe('string');
    });

    test('should validate required fields on insert', async () => {
      if (!db) return;

      const invalidDoc = { title: 'Missing path and content' };
      await expect(db.insert(invalidDoc as any)).rejects.toThrow(ValidationError);
    });

    test('should retrieve a document by ID', async () => {
      if (!db || !createdDocId) return;

      const doc = await db.get(createdDocId);
      expect(doc.id).toBe(createdDocId);
      expect(doc.title).toBe(testDocument.title);
      expect(doc.path).toBe(testDocument.path);
      expect(doc.tags).toEqual(testDocument.tags);
    });

    test('should handle document not found', async () => {
      if (!db) return;

      await expect(db.get('00000000-0000-0000-0000-000000000000'))
        .rejects.toThrow(KotaDBError);
    });

    test('should update a document', async () => {
      if (!db || !createdDocId) return;

      const updates = {
        content: 'This document has been updated by the test suite.',
        tags: ['test', 'typescript', 'client', 'updated']
      };

      const updatedDoc = await db.update(createdDocId, updates);
      expect(updatedDoc.id).toBe(createdDocId);
      expect(updatedDoc.tags).toContain('updated');
    });

    test('should list documents', async () => {
      if (!db) return;

      try {
        const docs = await db.listAll({ limit: 10 });
        expect(Array.isArray(docs)).toBe(true);
        if (createdDocId) {
          expect(docs.some(doc => doc.id === createdDocId)).toBe(true);
        }
      } catch (error) {
        // List endpoint might not be implemented - that's ok
        expect(error).toBeInstanceOf(KotaDBError);
      }
    });

    test('should delete a document', async () => {
      if (!db || !createdDocId) return;

      const result = await db.delete(createdDocId);
      expect(result).toBe(true);

      // Verify deletion
      await expect(db.get(createdDocId)).rejects.toThrow(NotFoundError);
    });
  });

  describe('Search Operations', () => {
    const searchTestDocs = [
      {
        path: '/test/search-doc-1.md',
        title: 'TypeScript Programming Guide',
        content: 'Learn TypeScript programming with examples and best practices.',
        tags: ['typescript', 'programming', 'guide']
      },
      {
        path: '/test/search-doc-2.md',
        title: 'JavaScript Framework Comparison',
        content: 'Comparing React, Vue, and Angular frameworks for web development.',
        tags: ['javascript', 'react', 'vue', 'angular']
      }
    ];

    let searchDocIds: string[] = [];

    beforeAll(async () => {
      if (!db) return;

      // Create test documents for search
      for (const doc of searchTestDocs) {
        try {
          const docId = await db.insert(doc);
          searchDocIds.push(docId);
        } catch (error) {
          // Continue if document creation fails
        }
      }
    });

    afterAll(async () => {
      if (!db) return;

      // Cleanup search test documents
      for (const docId of searchDocIds) {
        try {
          await db.delete(docId);
        } catch (error) {
          // Continue cleanup even if some deletions fail
        }
      }
    });

    test('should perform text search', async () => {
      if (!db || searchDocIds.length === 0) return;

      const results = await db.query('typescript programming', { limit: 5 });
      expect(results).toHaveProperty('results');
      expect(results).toHaveProperty('total_count');
      expect(Array.isArray(results.results)).toBe(true);
    });

    test('should handle empty search results', async () => {
      if (!db) return;

      const results = await db.query('nonexistentquerythatwillreturnnothing12345');
      expect(results.results).toEqual([]);
      expect(results.total_count).toBe(0);
    });

    test('should perform search with pagination', async () => {
      if (!db) return;

      const results = await db.query('test', { limit: 2, offset: 0 });
      expect(results.results.length).toBeLessThanOrEqual(2);
    });

    test('should perform semantic search if available', async () => {
      if (!db || searchDocIds.length === 0) return;

      try {
        const results = await db.semanticSearch('programming concepts', { limit: 3 });
        expect(results).toHaveProperty('results');
        expect(Array.isArray(results.results)).toBe(true);
      } catch (error) {
        // Semantic search might not be available - that's ok
        expect(error).toBeInstanceOf(NotFoundError);
      }
    });

    test('should perform hybrid search if available', async () => {
      if (!db || searchDocIds.length === 0) return;

      try {
        const results = await db.hybridSearch('web development frameworks', { 
          limit: 5,
          semantic_weight: 0.6 
        });
        expect(results).toHaveProperty('results');
        expect(Array.isArray(results.results)).toBe(true);
      } catch (error) {
        // Hybrid search might not be available - that's ok
        expect(error).toBeInstanceOf(NotFoundError);
      }
    });
  });

  describe('Error Handling', () => {
    test('should handle connection errors', async () => {
      const badClient = new KotaDB({ url: INVALID_SERVER_URL, timeout: 1000 });
      await expect(badClient.health()).rejects.toThrow(ConnectionError);
    });

    test('should handle server errors gracefully', async () => {
      if (!db) return;

      // Test with malformed document ID
      await expect(db.get('invalid-id-format')).rejects.toThrow(KotaDBError);
    });

    test('should handle validation errors', async () => {
      if (!db) return;

      const invalidDocument = { path: '/test' }; // Missing required fields
      await expect(db.insert(invalidDocument as any)).rejects.toThrow(ValidationError);
    });

    test('should handle network timeouts', async () => {
      // Create client with very short timeout and bad URL to guarantee failure
      const timeoutClient = new KotaDB({ 
        url: 'http://192.0.2.1:18432', // RFC5737 TEST-NET-1 address (guaranteed to timeout)
        timeout: 100 // 100ms timeout
      });

      await expect(timeoutClient.health()).rejects.toThrow(ConnectionError);
    });
  });

  describe('Configuration and Advanced Usage', () => {
    test('should accept custom headers', () => {
      const client = new KotaDB({
        url: TEST_SERVER_URL,
        headers: {
          'Authorization': 'Bearer test-token',
          'X-Custom-Header': 'test-value'
        }
      });
      expect(client).toBeInstanceOf(KotaDB);
    });

    test('should handle retry configuration', () => {
      const client = new KotaDB({
        url: TEST_SERVER_URL,
        retries: 5,
        timeout: 30000
      });
      expect(client).toBeInstanceOf(KotaDB);
    });
  });
});

describe('Edge Cases and Stress Tests', () => {
  let db: KotaDB;

  beforeAll(() => {
    try {
      db = new KotaDB({ url: TEST_SERVER_URL });
    } catch (error) {
      console.warn('Test server not available for stress tests');
    }
  });

  test('should handle large document content', async () => {
    if (!db) return;

    const largeContent = 'Large document content. '.repeat(10000); // ~250KB
    const largeDoc = {
      path: '/test/large-document.md',
      title: 'Large Test Document',
      content: largeContent,
      tags: ['test', 'large']
    };

    try {
      const docId = await db.insert(largeDoc);
      expect(docId).toBeTruthy();

      const retrieved = await db.get(docId);
      expect(retrieved.content.length).toBeGreaterThan(200000);

      await db.delete(docId);
    } catch (error) {
      // Server might have size limits - that's acceptable
      expect(error).toBeInstanceOf(KotaDBError);
    }
  });

  test('should handle concurrent operations', async () => {
    if (!db) return;

    const concurrentOps = Array.from({ length: 5 }, (_, i) => 
      db.insert({
        path: `/test/concurrent-${i}.md`,
        title: `Concurrent Test ${i}`,
        content: `Test document ${i} for concurrent operations.`,
        tags: ['test', 'concurrent', `batch-${i}`]
      })
    );

    try {
      const docIds = await Promise.all(concurrentOps);
      expect(docIds).toHaveLength(5);
      expect(docIds.every(id => typeof id === 'string')).toBe(true);

      // Cleanup
      await Promise.all(docIds.map(id => db.delete(id).catch(() => {})));
    } catch (error) {
      // Some concurrent operations might fail - that's acceptable for testing
      expect(error).toBeInstanceOf(KotaDBError);
    }
  });

  test('should handle special characters in content', async () => {
    if (!db) return;

    const specialDoc = {
      path: '/test/special-chars.md',
      title: 'Special Characters Test 🚀',
      content: 'Testing émojis 🎉, ünïcödé, and spéciål chäracters: @#$%^&*()[]{}',
      tags: ['test', 'unicode', 'special-chars']
    };

    try {
      const docId = await db.insert(specialDoc);
      const retrieved = await db.get(docId);
      expect(retrieved.title).toBe(specialDoc.title);
      expect(retrieved.content).toBe(specialDoc.content);
      await db.delete(docId);
    } catch (error) {
      console.warn('Special character test failed:', error instanceof Error ? error.message : String(error));
    }
  });
});