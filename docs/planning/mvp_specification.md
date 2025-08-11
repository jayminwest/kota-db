---
title: "KotaDB MVP Specification"
tags: [database, mvp, specification]
related: ["IMPLEMENTATION_PLAN.md", "TECHNICAL_ARCHITECTURE.md"]
key_concepts: [mvp, minimum-viable-product, quick-wins, pragmatic-design]
personal_contexts: []
created: 2025-07-02
updated: 2025-07-02
created_by: "Claude Code"
---

# KotaDB MVP Specification

## Overview

This document defines a Minimum Viable Product for KotaDB that can be built in 2-3 weeks and immediately provide value to KOTA. The MVP focuses on solving the most painful current problems while laying a foundation for future expansion.

## MVP Goals

1. **Eliminate startup scan time** (currently ~30s for 1000 files)
2. **Enable persistent indices** (survive restarts)
3. **Provide fast full-text search** (<10ms for common queries)
4. **Support basic relationship queries** (1-2 levels deep)
5. **Maintain Git compatibility** (keep markdown files as source)

## What's In Scope

### Core Features (Week 1)

1. **Document Storage**
   - Read markdown files on demand (not stored in DB)
   - Store only metadata and indices
   - SHA-256 hashes for change detection

2. **Primary Index**
   - Simple B-tree for path → metadata lookup
   - In-memory with periodic persistence
   - ~500 bytes per document overhead

3. **Full-Text Search**
   - Basic trigram index
   - Case-insensitive matching
   - Simple relevance scoring (TF-IDF)

4. **Tag Index**
   - Inverted index for tags
   - Fast intersection queries
   - Support for tag hierarchies

### Extended Features (Week 2)

5. **Relationship Graph**
   - Simple adjacency list
   - Bidirectional links
   - 1-2 level traversal only

6. **File Watcher**
   - Monitor for changes
   - Incremental index updates
   - Debouncing for rapid edits

7. **Basic Query Interface**
   - Simple JSON-based queries
   - No query language parser
   - Direct index access

### Integration (Week 3)

8. **CLI Commands**
   - `kota db index` - Build indices
   - `kota db search` - Query interface
   - `kota db stats` - Database statistics

9. **MCP Server**
   - Expose search via MCP tools
   - Replace KnowledgeOrgServer indices

10. **Migration Tool**
    - Scan existing files
    - Build initial indices
    - Verify integrity

## What's Out of Scope (Future)

- ❌ Complex query language (use JSON for now)
- ❌ Semantic/vector search (requires embeddings)
- ❌ Advanced graph algorithms (keep it simple)
- ❌ Compression (files stay uncompressed)
- ❌ Transactions (single-writer for now)
- ❌ Backup/restore (just rebuild indices)
- ❌ Encryption (rely on OS)

## Technical Design

### Storage Format

```rust
// Minimal document metadata
pub struct DocumentMeta {
    pub id: [u8; 16],        // UUID
    pub path: String,        // Full path
    pub hash: [u8; 32],      // Content hash
    pub size: u64,           // File size
    pub created: i64,        // Unix timestamp
    pub updated: i64,        // Unix timestamp
    pub title: String,       // From frontmatter
    pub word_count: u32,     // For scoring
}

// Simple index entry
pub struct IndexEntry {
    pub doc_id: [u8; 16],
    pub score: f32,          // Relevance score
}
```

### File Layout

```
~/.kota/db/
├── meta.db              # Document metadata (MessagePack)
├── indices/
│   ├── paths.idx        # Path → ID mapping
│   ├── trigrams.idx     # Trigram inverted index
│   ├── tags.idx         # Tag inverted index
│   └── links.idx        # Relationship graph
└── wal/                 # Write-ahead log
    └── changes.log      # Pending updates
```

### Index Structures

#### Path Index (B-Tree)
```rust
// Simple B-tree node
pub struct BTreeNode {
    pub keys: Vec<String>,      // Paths
    pub values: Vec<[u8; 16]>,  // Document IDs
    pub children: Vec<u64>,     // Child page offsets
    pub is_leaf: bool,
}
```

#### Trigram Index
```rust
// Trigram posting list
pub struct TrigramIndex {
    // Trigram → Document IDs
    pub postings: HashMap<[u8; 3], Vec<[u8; 16]>>,
    
    // Document → Trigram positions
    pub positions: HashMap<[u8; 16], Vec<u32>>,
}
```

#### Tag Index
```rust
// Simple inverted index
pub struct TagIndex {
    // Tag → Document IDs
    pub postings: HashMap<String, Vec<[u8; 16]>>,
    
    // Document → Tags (for removal)
    pub doc_tags: HashMap<[u8; 16], Vec<String>>,
}
```

### Query Format

Simple JSON-based queries:

```json
// Text search
{
  "type": "text",
  "query": "rust programming",
  "limit": 10
}

// Tag filter
{
  "type": "tags",
  "tags": ["meeting", "cogzia"],
  "op": "and"
}

// Combined query
{
  "type": "and",
  "queries": [
    { "type": "text", "query": "consciousness" },
    { "type": "tags", "tags": ["philosophy"] }
  ]
}

// Relationship query
{
  "type": "related",
  "start": "/projects/kota-ai/README.md",
  "depth": 1
}
```

## Implementation Plan

### Week 1: Core Storage and Indexing

#### Day 1-2: Storage Layer
```rust
// Minimal implementation
pub struct Storage {
    meta: HashMap<[u8; 16], DocumentMeta>,
    path_index: BTreeMap<String, [u8; 16]>,
}

impl Storage {
    pub fn insert(&mut self, path: &str, meta: DocumentMeta);
    pub fn get(&self, id: &[u8; 16]) -> Option<&DocumentMeta>;
    pub fn persist(&self) -> Result<()>;
    pub fn load() -> Result<Self>;
}
```

#### Day 3-4: Trigram Index
```rust
pub struct TrigramIndex {
    postings: HashMap<[u8; 3], RoaringBitmap>,
}

impl TrigramIndex {
    pub fn index_document(&mut self, id: [u8; 16], content: &str);
    pub fn search(&self, query: &str) -> Vec<[u8; 16]>;
}
```

#### Day 5: Tag Index
```rust
pub struct TagIndex {
    postings: HashMap<String, RoaringBitmap>,
}

impl TagIndex {
    pub fn add_tags(&mut self, id: [u8; 16], tags: &[String]);
    pub fn search(&self, tags: &[String]) -> Vec<[u8; 16]>;
}
```

### Week 2: Extended Features

#### Day 6-7: Relationship Graph
```rust
pub struct GraphIndex {
    edges: HashMap<[u8; 16], Vec<[u8; 16]>>,
}

impl GraphIndex {
    pub fn add_edge(&mut self, from: [u8; 16], to: [u8; 16]);
    pub fn get_related(&self, id: [u8; 16], depth: u32) -> Vec<[u8; 16]>;
}
```

#### Day 8-9: File Watcher
```rust
pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    db: Arc<Mutex<Database>>,
}

impl FileWatcher {
    pub fn watch(&mut self, path: &Path) -> Result<()>;
    pub fn handle_event(&mut self, event: notify::Event);
}
```

#### Day 10: Query Engine
```rust
pub struct QueryEngine {
    storage: Arc<Storage>,
    indices: Indices,
}

impl QueryEngine {
    pub fn execute(&self, query: Query) -> Result<Vec<SearchResult>>;
}
```

### Week 3: Integration

#### Day 11-12: CLI Integration
```bash
# New commands
kota db index                 # Build/rebuild indices
kota db search "query"        # Search interface
kota db stats                 # Show statistics
kota db verify                # Check integrity
```

#### Day 13-14: MCP Server
```rust
pub struct DatabaseServer {
    db: Arc<Database>,
}

impl McpServer for DatabaseServer {
    async fn handle_tool_call(&self, tool: &str, args: Value) -> Result<Value> {
        match tool {
            "search" => self.search(args).await,
            "get_related" => self.get_related(args).await,
            _ => Err(anyhow!("Unknown tool")),
        }
    }
}
```

#### Day 15: Testing and Polish
- Integration tests
- Performance benchmarks
- Documentation
- Bug fixes

## Performance Targets

### Storage
- **Metadata size**: <500 bytes per document
- **Index size**: <2KB per document total
- **Memory usage**: <100MB for 10k documents

### Operations
- **Indexing**: >1000 documents/second
- **Search latency**: <10ms for simple queries
- **Startup time**: <100ms (with indices)
- **Update latency**: <1ms per document

### Benchmarks
```rust
#[bench]
fn bench_index_document(b: &mut Bencher) {
    let mut idx = TrigramIndex::new();
    b.iter(|| {
        idx.index_document(uuid::Uuid::new_v4().into(), "sample content");
    });
}

#[bench]
fn bench_search(b: &mut Bencher) {
    let idx = create_test_index();
    b.iter(|| {
        idx.search("test query");
    });
}
```

## Migration Path

### From Current System

1. **Parallel Operation**
   - Run alongside existing KnowledgeOrgServer
   - Compare results for validation
   - Gradual cutover

2. **Data Migration**
   ```rust
   pub async fn migrate(source: &Path) -> Result<()> {
       let db = Database::new()?;
       
       for entry in WalkDir::new(source) {
           let path = entry?.path();
           if path.extension() == Some("md") {
               db.index_file(path).await?;
           }
       }
       
       db.persist()?;
       Ok(())
   }
   ```

3. **Verification**
   - Count documents
   - Verify relationships
   - Test queries
   - Check performance

## Success Criteria

### Functional
- ✅ Indexes persist between restarts
- ✅ Search returns correct results
- ✅ File changes are detected
- ✅ Relationships are bidirectional
- ✅ No data corruption

### Performance
- ✅ Startup time <1 second
- ✅ Search latency <10ms
- ✅ Memory usage <100MB
- ✅ CPU usage minimal when idle

### Integration
- ✅ CLI commands work correctly
- ✅ MCP server responds properly
- ✅ No regression in functionality
- ✅ Easy to set up and use

## Risk Mitigation

### Technical Risks
1. **Corruption**: Use checksums, atomic writes
2. **Performance**: Profile early, optimize hotspots
3. **Compatibility**: Keep markdown files unchanged

### Schedule Risks
1. **Scope creep**: Stick to MVP features
2. **Integration issues**: Test continuously
3. **Unknown unknowns**: Time buffer in week 3

## Future Roadmap

After MVP success:

### Phase 2 (Weeks 4-6)
- Query language parser
- Advanced text search (stemming, synonyms)
- Basic vector search
- Compression

### Phase 3 (Weeks 7-9)
- ACID transactions
- Multi-version concurrency
- Advanced graph algorithms
- Backup/restore

### Phase 4 (Weeks 10-12)
- Distributed queries
- Real-time subscriptions
- Machine learning integration
- Performance optimization

## Conclusion

This MVP provides immediate value by solving KOTA's most pressing database needs while laying a foundation for future enhancements. The 3-week timeline is aggressive but achievable by focusing on pragmatic solutions and deferring complexity.

The key is to start simple, validate the approach, and iterate based on real usage. This MVP will prove the custom database concept and provide a platform for the more ambitious features described in the full implementation plan.