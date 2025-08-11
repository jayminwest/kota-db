/**
 * KotaDB TypeScript client types and interfaces.
 */

export interface Document {
  id: string;
  path: string;
  title: string;
  content: number[] | string; // Server returns byte array, but we can also accept string
  content_hash?: string;
  size_bytes?: number;
  tags: string[];
  created_at: number; // Unix timestamp
  modified_at?: number; // Unix timestamp  
  word_count?: number;
  metadata?: Record<string, any>;
}

export interface SearchResult {
  document: Document;
  score: number;
  content_preview: string;
}

export interface QueryResult {
  results: SearchResult[];
  total_count: number;
  query_time_ms: number;
}

export interface CreateDocumentRequest {
  path: string;
  title: string;
  content: string | number[]; // Accept string or byte array
  tags?: string[];
  metadata?: Record<string, any>;
}

export interface UpdateDocumentRequest {
  path?: string;
  title?: string;
  content?: string | number[]; // Accept string or byte array
  tags?: string[];
  metadata?: Record<string, any>;
}

export interface SearchOptions {
  limit?: number;
  offset?: number;
}

export interface SemanticSearchOptions extends SearchOptions {
  model?: string;
}

export interface HybridSearchOptions extends SearchOptions {
  semantic_weight?: number;
}

export interface ConnectionConfig {
  url?: string;
  timeout?: number;
  retries?: number;
  headers?: Record<string, string>;
}

export interface HealthStatus {
  status: string;
  version?: string;
  uptime?: number;
  [key: string]: any;
}

export interface DatabaseStats {
  document_count?: number;
  total_size_bytes?: number;
  index_count?: number;
  [key: string]: any;
}

// Error types
export class KotaDBError extends Error {
  constructor(message: string, public statusCode?: number, public responseBody?: string) {
    super(message);
    this.name = 'KotaDBError';
  }
}

export class ConnectionError extends KotaDBError {
  constructor(message: string) {
    super(message);
    this.name = 'ConnectionError';
  }
}

export class ValidationError extends KotaDBError {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

export class NotFoundError extends KotaDBError {
  constructor(message: string = 'Resource not found') {
    super(message, 404);
    this.name = 'NotFoundError';
  }
}

export class ServerError extends KotaDBError {
  constructor(message: string, statusCode?: number, responseBody?: string) {
    super(message, statusCode, responseBody);
    this.name = 'ServerError';
  }
}

// Type aliases for convenience
export type DocumentInput = CreateDocumentRequest;
export type DocumentUpdate = UpdateDocumentRequest;
export type ConnectionString = string;
