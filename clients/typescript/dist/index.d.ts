/**
 * KotaDB TypeScript/JavaScript Client
 *
 * A simple HTTP client for KotaDB that provides PostgreSQL-level ease of use.
 *
 * @example
 * ```typescript
 * import { KotaDB } from 'kotadb-client';
 *
 * const db = new KotaDB({ url: 'http://localhost:8080' });
 * const results = await db.query('rust patterns');
 * const docId = await db.insert({
 *   path: '/notes/meeting.md',
 *   title: 'My Note',
 *   content: '...',
 *   tags: ['work']
 * });
 * ```
 */
export { KotaDB, connect } from './client';
export * from './types';
export * from './validated-types';
export { validateFilePath, validateDirectoryPath, validateDocumentId, validateTitle, validateTag, validateSearchQuery, validateTimestamp, validateSize } from './validation';
export * from './builders';
import { KotaDB } from './client';
export default KotaDB;
//# sourceMappingURL=index.d.ts.map