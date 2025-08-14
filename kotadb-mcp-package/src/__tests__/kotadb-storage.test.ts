import { KotaDBStorage } from '../kotadb-storage';
import * as fs from 'fs/promises';
import * as path from 'path';
import { tmpdir } from 'os';

describe('KotaDBStorage', () => {
  let storage: KotaDBStorage;
  let tempDir: string;

  beforeEach(async () => {
    // Create a temporary directory for testing
    tempDir = await fs.mkdtemp(path.join(tmpdir(), 'kotadb-test-'));
    storage = new KotaDBStorage(tempDir);
    await storage.initialize();
  });

  afterEach(async () => {
    // Clean up temporary directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch (error) {
      // Ignore cleanup errors
      console.warn('Failed to cleanup test directory:', error);
    }
  });

  describe('Document Creation', () => {
    test('should create a document with all fields', async () => {
      const doc = await storage.createDocument({
        path: '/test.md',
        title: 'Test Document',
        content: 'This is a test document',
        tags: ['test', 'example'],
      });

      expect(doc.id).toBeDefined();
      expect(doc.path).toBe('/test.md');
      expect(doc.title).toBe('Test Document');
      expect(doc.content).toBe('This is a test document');
      expect(doc.tags).toEqual(['test', 'example']);
      expect(doc.createdAt).toBeDefined();
      expect(doc.updatedAt).toBeDefined();
    });

    test('should create a document with minimal fields', async () => {
      const doc = await storage.createDocument({
        path: '/minimal.md',
        content: 'Minimal content',
      });

      expect(doc.id).toBeDefined();
      expect(doc.path).toBe('/minimal.md');
      expect(doc.title).toBe('minimal'); // Auto-generated from path
      expect(doc.content).toBe('Minimal content');
      expect(doc.tags).toEqual([]);
      expect(doc.createdAt).toBeDefined();
    });

    test('should persist document to filesystem', async () => {
      const doc = await storage.createDocument({
        path: '/persistent.md',
        title: 'Persistent Doc',
        content: 'This should persist',
      });

      // Check that markdown file exists
      const markdownPath = path.join(tempDir, `${doc.id}.md`);
      const markdownExists = await fs.access(markdownPath).then(() => true).catch(() => false);
      expect(markdownExists).toBe(true);

      // Check markdown content
      const markdownContent = await fs.readFile(markdownPath, 'utf-8');
      expect(markdownContent).toContain('# Persistent Doc');
      expect(markdownContent).toContain('This should persist');
    });
  });

  describe('Document Retrieval', () => {
    test('should retrieve an existing document', async () => {
      const created = await storage.createDocument({
        path: '/retrieve-test.md',
        title: 'Retrieve Test',
        content: 'Content to retrieve',
      });

      const retrieved = await storage.getDocument(created.id);
      expect(retrieved).toBeDefined();
      expect(retrieved!.id).toBe(created.id);
      expect(retrieved!.title).toBe('Retrieve Test');
      expect(retrieved!.content).toBe('Content to retrieve');
    });

    test('should return null for non-existent document', async () => {
      const result = await storage.getDocument('non-existent-id');
      expect(result).toBeNull();
    });
  });

  describe('Document Updates', () => {
    test('should update document content', async () => {
      const doc = await storage.createDocument({
        path: '/update-test.md',
        content: 'Original content',
      });

      const updated = await storage.updateDocument(doc.id, 'Updated content');
      expect(updated).toBeDefined();
      expect(updated!.content).toBe('Updated content');
      expect(updated!.updatedAt).not.toBe(doc.updatedAt);
    });

    test('should return null when updating non-existent document', async () => {
      const result = await storage.updateDocument('non-existent', 'New content');
      expect(result).toBeNull();
    });
  });

  describe('Document Deletion', () => {
    test('should delete an existing document', async () => {
      const doc = await storage.createDocument({
        path: '/delete-test.md',
        content: 'To be deleted',
      });

      const deleted = await storage.deleteDocument(doc.id);
      expect(deleted).toBe(true);

      // Verify document is gone
      const retrieved = await storage.getDocument(doc.id);
      expect(retrieved).toBeNull();
    });

    test('should return false when deleting non-existent document', async () => {
      const result = await storage.deleteDocument('non-existent');
      expect(result).toBe(false);
    });
  });

  describe('Document Listing', () => {
    test('should list all documents', async () => {
      await storage.createDocument({ path: '/doc1.md', content: 'Content 1' });
      await storage.createDocument({ path: '/doc2.md', content: 'Content 2' });
      await storage.createDocument({ path: '/doc3.md', content: 'Content 3' });

      const result = await storage.listDocuments();
      expect(result.documents).toHaveLength(3);
      expect(result.total).toBe(3);
    });

    test('should support pagination', async () => {
      await storage.createDocument({ path: '/doc1.md', content: 'Content 1' });
      await storage.createDocument({ path: '/doc2.md', content: 'Content 2' });
      await storage.createDocument({ path: '/doc3.md', content: 'Content 3' });

      const result = await storage.listDocuments(2, 1);
      expect(result.documents).toHaveLength(2);
      expect(result.total).toBe(3);
    });
  });

  describe('Document Search', () => {
    beforeEach(async () => {
      await storage.createDocument({
        path: '/rust-guide.md',
        title: 'Rust Programming Guide',
        content: 'Learn Rust programming with examples and best practices',
        tags: ['rust', 'programming'],
      });
      await storage.createDocument({
        path: '/javascript-tips.md',
        title: 'JavaScript Tips',
        content: 'Useful JavaScript tips and tricks for developers',
        tags: ['javascript', 'web'],
      });
      await storage.createDocument({
        path: '/programming-basics.md',
        title: 'Programming Basics',
        content: 'Basic programming concepts for beginners',
        tags: ['programming', 'basics'],
      });
    });

    test('should find documents by content', async () => {
      const results = await storage.searchDocuments('JavaScript', 10);
      expect(results).toHaveLength(1);
      expect(results[0].title).toBe('JavaScript Tips');
      expect(results[0].score).toBeGreaterThan(0);
    });

    test('should find documents by title', async () => {
      const results = await storage.searchDocuments('Rust', 10);
      expect(results).toHaveLength(1);
      expect(results[0].title).toBe('Rust Programming Guide');
      expect(results[0].score).toBeGreaterThan(0);
    });

    test('should find documents by tags', async () => {
      const results = await storage.searchDocuments('programming', 10);
      expect(results.length).toBeGreaterThanOrEqual(2);
      
      const titles = results.map(r => r.title);
      expect(titles).toContain('Rust Programming Guide');
      expect(titles).toContain('Programming Basics');
    });

    test('should return empty results for non-matching query', async () => {
      const results = await storage.searchDocuments('nonexistent', 10);
      expect(results).toHaveLength(0);
    });

    test('should respect search limit', async () => {
      const results = await storage.searchDocuments('programming', 1);
      expect(results).toHaveLength(1);
    });

    test('should include content preview in results', async () => {
      const results = await storage.searchDocuments('JavaScript', 10);
      expect(results[0].content_preview).toBeDefined();
      expect(results[0].content_preview).toContain('JavaScript');
    });
  });

  describe('Statistics', () => {
    test('should return correct stats for empty database', async () => {
      const stats = await storage.getStats();
      expect(stats.total_documents).toBe(0);
      expect(stats.total_size_bytes).toBe(0);
      expect(stats.data_directory).toBe(tempDir);
    });

    test('should return correct stats with documents', async () => {
      await storage.createDocument({
        path: '/stats-test.md',
        title: 'Stats Test',
        content: 'Content for statistics testing',
      });

      const stats = await storage.getStats();
      expect(stats.total_documents).toBe(1);
      expect(stats.total_size_bytes).toBeGreaterThan(0);
      expect(stats.data_directory).toBe(tempDir);
    });
  });

  describe('Persistence', () => {
    test('should persist documents across storage instances', async () => {
      // Create document with first storage instance
      const doc = await storage.createDocument({
        path: '/persistence-test.md',
        title: 'Persistence Test',
        content: 'This should persist',
      });

      // Create new storage instance with same directory
      const newStorage = new KotaDBStorage(tempDir);
      await newStorage.initialize();

      // Should be able to retrieve the document
      const retrieved = await newStorage.getDocument(doc.id);
      expect(retrieved).toBeDefined();
      expect(retrieved!.title).toBe('Persistence Test');
      expect(retrieved!.content).toBe('This should persist');
    });
  });
});