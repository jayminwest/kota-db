/**
 * Builder Patterns Tests for KotaDB TypeScript Client
 * 
 * Comprehensive tests for builder patterns that ensure fluent, safe
 * construction of documents and queries with validation at each step.
 */

import {
  DocumentBuilder,
  QueryBuilder,
  UpdateBuilder
} from '../src/builders';
import {
  ValidatedPath,
  ValidatedDocumentId,
  ValidatedTitle
} from '../src/validated-types';
import { ValidationError } from '../src/validation';

describe('DocumentBuilder', () => {
  test('should build valid document with all fields', () => {
    const builder = new DocumentBuilder()
      .path('/notes/meeting.md')
      .title('Team Meeting Notes')
      .content('Meeting content here...')
      .addTag('work')
      .addTag('meeting')
      .addMetadata('priority', 'high')
      .addMetadata('author', 'john.doe');

    const document = builder.build();

    expect(document.path).toBe('/notes/meeting.md');
    expect(document.title).toBe('Team Meeting Notes');
    expect(document.content).toBe('Meeting content here...');
    expect(document.tags).toEqual(['work', 'meeting']);
    expect(document.metadata).toEqual({
      priority: 'high',
      author: 'john.doe'
    });
  });

  test('should build document with validated types', () => {
    const validatedPath = new ValidatedPath('/notes/validated.md');
    const validatedTitle = new ValidatedTitle('Validated Document');
    const validatedId = ValidatedDocumentId.new();

    const document = new DocumentBuilder()
      .path(validatedPath)
      .title(validatedTitle)
      .content('Content from validated types')
      .id(validatedId)
      .build();

    expect(document.path).toBe('/notes/validated.md');
    expect(document.title).toBe('Validated Document');
    expect(document.content).toBe('Content from validated types');
  });

  test('should generate auto ID', () => {
    const builder = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content('Content')
      .autoId();

    const document1 = builder.buildWithTimestamps();
    const document2 = builder.buildWithTimestamps();

    expect(document1.id).toBeTruthy();
    expect(document2.id).toBeTruthy();
    expect(document1.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  });

  test('should handle array content', () => {
    const arrayContent = [72, 101, 108, 108, 111]; // "Hello" in bytes

    const document = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content(arrayContent)
      .build();

    expect(document.content).toEqual(arrayContent);
  });

  test('should build document with timestamps', () => {
    const beforeTime = Math.floor(Date.now() / 1000);

    const document = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content('Test content')
      .buildWithTimestamps();

    const afterTime = Math.floor(Date.now() / 1000);

    expect(document.created_at).toBeGreaterThanOrEqual(beforeTime);
    expect(document.created_at).toBeLessThanOrEqual(afterTime);
    expect(document.modified_at).toBeGreaterThanOrEqual(beforeTime);
    expect(document.modified_at).toBeLessThanOrEqual(afterTime);
    expect(document.size_bytes).toBeGreaterThan(0);
  });

  test('should calculate content size correctly', () => {
    const stringContent = 'Hello, World!';
    const document = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content(stringContent)
      .buildWithTimestamps();

    expect(document.size_bytes).toBe(new TextEncoder().encode(stringContent).length);
  });

  test('should prevent duplicate tags', () => {
    const document = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content('Content')
      .addTag('duplicate')
      .addTag('duplicate')
      .addTag('unique')
      .build();

    expect(document.tags).toEqual(['duplicate', 'unique']);
  });

  test('should replace tags when using tags() method', () => {
    const document = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content('Content')
      .addTag('old-tag')
      .tags(['new', 'tags', 'only'])
      .build();

    expect(document.tags).toEqual(['new', 'tags', 'only']);
  });

  test('should replace metadata when using metadata() method', () => {
    const document = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content('Content')
      .addMetadata('old', 'value')
      .metadata({ new: 'metadata', only: true })
      .build();

    expect(document.metadata).toEqual({ new: 'metadata', only: true });
  });

  test('should validate path when building', () => {
    const builder = new DocumentBuilder()
      .title('Test')
      .content('Content');

    expect(() => builder.build()).toThrow(ValidationError);
    expect(() => builder.build()).toThrow('Document path is required');
  });

  test('should validate title when building', () => {
    const builder = new DocumentBuilder()
      .path('/test.md')
      .content('Content');

    expect(() => builder.build()).toThrow(ValidationError);
    expect(() => builder.build()).toThrow('Document title is required');
  });

  test('should validate content when building', () => {
    const builder = new DocumentBuilder()
      .path('/test.md')
      .title('Test');

    expect(() => builder.build()).toThrow(ValidationError);
    expect(() => builder.build()).toThrow('Document content is required');
  });

  test('should validate tags', () => {
    const builder = new DocumentBuilder()
      .path('/test.md')
      .title('Test')
      .content('Content');

    expect(() => builder.addTag('')).toThrow(ValidationError);
    expect(() => builder.addTag('invalid@tag')).toThrow(ValidationError);
    expect(() => builder.tags(['valid', 'invalid@tag'])).toThrow(ValidationError);
  });

  test('should validate path input', () => {
    const builder = new DocumentBuilder();

    expect(() => builder.path('../../../etc/passwd')).toThrow(ValidationError);
    expect(() => builder.path('/path\x00injection')).toThrow(ValidationError);
    expect(() => builder.path('CON.txt')).toThrow(ValidationError);
  });

  test('should validate title input', () => {
    const builder = new DocumentBuilder();

    expect(() => builder.title('')).toThrow(ValidationError);
    expect(() => builder.title('   ')).toThrow(ValidationError);
    expect(() => builder.title('A'.repeat(1025))).toThrow(ValidationError);
  });
});

describe('QueryBuilder', () => {
  test('should build basic query', () => {
    const query = new QueryBuilder()
      .text('search terms')
      .limit(10)
      .offset(5)
      .build();

    expect(query.q).toBe('search terms');
    expect(query.limit).toBe(10);
    expect(query.offset).toBe(5);
  });

  test('should build query with semantic weight', () => {
    const query = new QueryBuilder()
      .text('semantic search')
      .semanticWeight(0.7)
      .limit(5)
      .build();

    expect(query.q).toBe('semantic search');
    expect(query.semantic_weight).toBe(0.7);
    expect(query.limit).toBe(5);
  });

  test('should add filters', () => {
    const query = new QueryBuilder()
      .text('filtered search')
      .addFilter('custom', 'value')
      .tagFilter('important')
      .pathFilter('/notes/*')
      .build();

    expect(query.q).toBe('filtered search');
    expect(query.custom).toBe('value');
    expect(query.tag).toBe('important');
    expect(query.path).toBe('/notes/*');
  });

  test('should build for semantic search', () => {
    const data = new QueryBuilder()
      .text('semantic query')
      .limit(15)
      .offset(10)
      .addFilter('model', 'openai')
      .buildForSemantic();

    expect(data.query).toBe('semantic query');
    expect(data.limit).toBe(15);
    expect(data.offset).toBe(10);
    expect(data.model).toBe('openai');
    expect(data.q).toBeUndefined(); // Should use 'query' not 'q'
  });

  test('should build for hybrid search', () => {
    const data = new QueryBuilder()
      .text('hybrid query')
      .semanticWeight(0.6)
      .limit(20)
      .buildForHybrid();

    expect(data.query).toBe('hybrid query');
    expect(data.semantic_weight).toBe(0.6);
    expect(data.limit).toBe(20);
  });

  test('should not include offset when zero', () => {
    const query = new QueryBuilder()
      .text('no offset')
      .offset(0)
      .build();

    expect(query.offset).toBeUndefined();
  });

  test('should include offset when greater than zero', () => {
    const query = new QueryBuilder()
      .text('with offset')
      .offset(5)
      .build();

    expect(query.offset).toBe(5);
  });

  test('should validate query text is required', () => {
    const builder = new QueryBuilder().limit(10);

    expect(() => builder.build()).toThrow(ValidationError);
    expect(() => builder.build()).toThrow('Query text is required');
    expect(() => builder.buildForSemantic()).toThrow(ValidationError);
    expect(() => builder.buildForHybrid()).toThrow(ValidationError);
  });

  test('should validate search query text', () => {
    const builder = new QueryBuilder();

    expect(() => builder.text('')).toThrow(ValidationError);
    expect(() => builder.text('   ')).toThrow(ValidationError);
    expect(() => builder.text('A'.repeat(1025))).toThrow(ValidationError);
  });

  test('should validate limit', () => {
    const builder = new QueryBuilder().text('valid query');

    expect(() => builder.limit(0)).toThrow(ValidationError);
    expect(() => builder.limit(-1)).toThrow(ValidationError);
    expect(() => builder.limit(10001)).toThrow(ValidationError);
  });

  test('should validate offset', () => {
    const builder = new QueryBuilder().text('valid query');

    expect(() => builder.offset(-1)).toThrow(ValidationError);
    expect(() => builder.offset(-100)).toThrow(ValidationError);
  });

  test('should validate semantic weight', () => {
    const builder = new QueryBuilder().text('valid query');

    expect(() => builder.semanticWeight(-0.1)).toThrow(ValidationError);
    expect(() => builder.semanticWeight(1.1)).toThrow(ValidationError);
    expect(() => builder.semanticWeight(2.0)).toThrow(ValidationError);
  });

  test('should validate tag filters', () => {
    const builder = new QueryBuilder().text('valid query');

    expect(() => builder.tagFilter('')).toThrow(ValidationError);
    expect(() => builder.tagFilter('invalid@tag')).toThrow(ValidationError);
  });

  test('should allow valid semantic weights', () => {
    const builder = new QueryBuilder().text('valid query');

    expect(() => builder.semanticWeight(0.0)).not.toThrow();
    expect(() => builder.semanticWeight(0.5)).not.toThrow();
    expect(() => builder.semanticWeight(1.0)).not.toThrow();
  });
});

describe('UpdateBuilder', () => {
  test('should build basic updates', () => {
    const updates = new UpdateBuilder()
      .title('Updated Title')
      .content('Updated content')
      .build();

    expect(updates.title).toBe('Updated Title');
    expect(updates.content).toBe('Updated content');
  });

  test('should handle tag operations', () => {
    const updates = new UpdateBuilder()
      .addTag('new-tag')
      .addTag('another-tag')
      .removeTag('old-tag')
      .build();

    expect(updates._tag_operations).toEqual({
      add: ['new-tag', 'another-tag'],
      remove: ['old-tag']
    });
  });

  test('should handle metadata operations', () => {
    const updates = new UpdateBuilder()
      .addMetadata('key1', 'value1')
      .addMetadata('key2', 42)
      .removeMetadata('old-key')
      .build();

    expect(updates._metadata_operations).toEqual({
      key1: 'value1',
      key2: 42,
      'old-key': null
    });
  });

  test('should replace tags completely', () => {
    const updates = new UpdateBuilder()
      .addTag('will-be-replaced')
      .replaceTags(['new', 'complete', 'list'])
      .build();

    expect(updates.tags).toEqual(['new', 'complete', 'list']);
    expect(updates._tag_operations).toBeUndefined();
  });

  test('should prevent duplicate tags in add operations', () => {
    const updates = new UpdateBuilder()
      .addTag('duplicate')
      .addTag('duplicate')
      .addTag('unique')
      .build();

    expect(updates._tag_operations.add).toEqual(['duplicate', 'unique']);
  });

  test('should prevent duplicate tags in remove operations', () => {
    const updates = new UpdateBuilder()
      .removeTag('duplicate')
      .removeTag('duplicate')
      .removeTag('unique')
      .build();

    expect(updates._tag_operations.remove).toEqual(['duplicate', 'unique']);
  });

  test('should validate title input', () => {
    const builder = new UpdateBuilder();

    expect(() => builder.title('')).toThrow(ValidationError);
    expect(() => builder.title('   ')).toThrow(ValidationError);
    expect(() => builder.title('A'.repeat(1025))).toThrow(ValidationError);
  });

  test('should validate tag inputs', () => {
    const builder = new UpdateBuilder();

    expect(() => builder.addTag('')).toThrow(ValidationError);
    expect(() => builder.addTag('invalid@tag')).toThrow(ValidationError);
    expect(() => builder.replaceTags(['valid', 'invalid@tag'])).toThrow(ValidationError);
  });

  test('should work with validated types', () => {
    const validatedTitle = new ValidatedTitle('Validated Updated Title');

    const updates = new UpdateBuilder()
      .title(validatedTitle)
      .build();

    expect(updates.title).toBe('Validated Updated Title');
  });

  test('should handle mixed content types', () => {
    const stringContent = 'String content';
    const arrayContent = [72, 101, 108, 108, 111]; // "Hello"

    const updates1 = new UpdateBuilder().content(stringContent).build();
    const updates2 = new UpdateBuilder().content(arrayContent).build();

    expect(updates1.content).toBe(stringContent);
    expect(updates2.content).toEqual(arrayContent);
  });

  test('should clear tag modifications when replacing', () => {
    const updates = new UpdateBuilder()
      .addTag('to-add')
      .removeTag('to-remove')
      .replaceTags(['replacement', 'tags'])
      .build();

    expect(updates.tags).toEqual(['replacement', 'tags']);
    expect(updates._tag_operations).toBeUndefined();
  });

  test('should handle empty builders', () => {
    const updates = new UpdateBuilder().build();
    expect(Object.keys(updates)).toHaveLength(0);
  });
});

describe('Builder Integration Tests', () => {
  test('should chain multiple builder operations fluently', () => {
    const document = new DocumentBuilder()
      .path('/integration/test.md')
      .title('Integration Test Document')
      .content('This tests fluent chaining')
      .addTag('integration')
      .addTag('test')
      .addMetadata('test_type', 'integration')
      .addMetadata('priority', 'high')
      .autoId()
      .build();

    expect(document.path).toBe('/integration/test.md');
    expect(document.title).toBe('Integration Test Document');
    expect(document.content).toBe('This tests fluent chaining');
    expect(document.tags).toEqual(['integration', 'test']);
    expect(document.metadata).toEqual({
      test_type: 'integration',
      priority: 'high'
    });
  });

  test('should handle complex query building', () => {
    const query = new QueryBuilder()
      .text('complex search terms')
      .limit(50)
      .offset(100)
      .semanticWeight(0.8)
      .tagFilter('important')
      .pathFilter('/documents/*')
      .addFilter('author', 'john.doe')
      .addFilter('date_range', '2023-01-01:2023-12-31')
      .build();

    expect(query).toEqual({
      q: 'complex search terms',
      limit: 50,
      offset: 100,
      semantic_weight: 0.8,
      tag: 'important',
      path: '/documents/*',
      author: 'john.doe',
      date_range: '2023-01-01:2023-12-31'
    });
  });

  test('should handle complex update building', () => {
    const updates = new UpdateBuilder()
      .title('Completely Updated Document')
      .content('New content for the document')
      .addTag('updated')
      .addTag('modified')
      .removeTag('old')
      .removeTag('deprecated')
      .addMetadata('last_modified_by', 'test-suite')
      .addMetadata('version', 2)
      .removeMetadata('obsolete_field')
      .build();

    expect(updates.title).toBe('Completely Updated Document');
    expect(updates.content).toBe('New content for the document');
    expect(updates._tag_operations).toEqual({
      add: ['updated', 'modified'],
      remove: ['old', 'deprecated']
    });
    expect(updates._metadata_operations).toEqual({
      last_modified_by: 'test-suite',
      version: 2,
      obsolete_field: null
    });
  });

  test('should maintain validation across all builders', () => {
    // All builders should consistently validate inputs
    const invalidPath = '../../../etc/passwd';
    const invalidTitle = '';
    const invalidTag = 'invalid@tag';

    expect(() => new DocumentBuilder().path(invalidPath)).toThrow(ValidationError);
    expect(() => new DocumentBuilder().title(invalidTitle)).toThrow(ValidationError);
    expect(() => new DocumentBuilder().addTag(invalidTag)).toThrow(ValidationError);

    expect(() => new UpdateBuilder().title(invalidTitle)).toThrow(ValidationError);
    expect(() => new UpdateBuilder().addTag(invalidTag)).toThrow(ValidationError);

    expect(() => new QueryBuilder().tagFilter(invalidTag)).toThrow(ValidationError);
  });
});