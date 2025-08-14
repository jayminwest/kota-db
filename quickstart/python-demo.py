#!/usr/bin/env python3
"""
KotaDB Python Client Demo - 60 Second Quick Start
Demonstrates all core features with real operations.
"""

import os
import time
import json
from typing import List, Dict, Any

try:
    from kotadb import KotaDB, DocumentBuilder, QueryBuilder, ValidatedPath
except ImportError:
    print("❌ kotadb-client not found. Install with: pip install kotadb-client")
    exit(1)

def main():
    print("🚀 KotaDB Python Demo - All Core Features")
    print("=" * 50)
    
    # Connect to KotaDB server
    kotadb_url = os.getenv("KOTADB_URL", "http://localhost:8080")
    print(f"📡 Connecting to KotaDB at {kotadb_url}")
    
    db = KotaDB(kotadb_url)
    
    # Test connection
    try:
        stats = db.stats()
        print(f"✅ Connected! Database has {stats.get('document_count', 0)} documents")
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        return
    
    print("\n1️⃣ DOCUMENT CREATION (Builder Pattern)")
    print("-" * 40)
    
    # Create sample documents with builder pattern (type-safe)
    sample_docs = []
    
    # Document 1: Programming guide
    doc1_id = db.insert_with_builder(
        DocumentBuilder()
        .path(ValidatedPath("/guides/rust-ownership.md"))
        .title("Rust Ownership Guide")
        .content("""# Rust Ownership
        
Rust's ownership system ensures memory safety without garbage collection.

## Key Concepts:
- Each value has an owner
- Only one owner at a time  
- When owner goes out of scope, value is dropped

## Examples:
```rust
let s = String::from("hello");  // s owns the string
let s2 = s;                     // ownership moves to s2
// println!("{}", s);           // Error! s no longer valid
```
""")
        .add_tag("rust")
        .add_tag("programming")
        .add_tag("tutorial")
    )
    sample_docs.append(doc1_id)
    
    # Document 2: Meeting notes
    doc2_id = db.insert_with_builder(
        DocumentBuilder()
        .path(ValidatedPath("/meetings/2024-08-14-standup.md"))
        .title("Team Standup - Aug 14")
        .content("""# Daily Standup - August 14, 2024

## Attendees
- Alice (PM)
- Bob (Backend) 
- Carol (Frontend)

## Updates
- **Alice**: Working on sprint planning
- **Bob**: Implementing KotaDB integration
- **Carol**: Building UI components

## Blockers
- Waiting for database schema review

## Action Items
- [ ] Bob: Finish KotaDB demo by Friday
- [ ] Carol: Update component library
""")
        .add_tag("meeting")
        .add_tag("standup")
        .add_tag("team")
    )
    sample_docs.append(doc2_id)
    
    # Document 3: Personal note
    doc3_id = db.insert_with_builder(
        DocumentBuilder()
        .path(ValidatedPath("/personal/learning-notes.md"))
        .title("Database Learning Notes")
        .content("""# Database Learning Notes

## KotaDB Features
- Custom storage engine
- Multiple index types (B+tree, trigram, vector)
- ACID compliance with WAL
- Zero external dependencies

## Performance
- Sub-10ms query latency
- 3,600+ operations per second
- Efficient memory usage

## Use Cases
- Personal knowledge bases
- Document management
- Search applications
- AI-powered systems
""")
        .add_tag("database")
        .add_tag("learning")
        .add_tag("personal")
    )
    sample_docs.append(doc3_id)
    
    print(f"✅ Created {len(sample_docs)} documents with builder pattern")
    
    print("\n2️⃣ DOCUMENT RETRIEVAL")
    print("-" * 40)
    
    # Get documents back
    for i, doc_id in enumerate(sample_docs, 1):
        try:
            doc = db.get(doc_id)
            print(f"📄 Doc {i}: '{doc['title']}' - {len(doc['content'])} chars")
        except Exception as e:
            print(f"❌ Failed to retrieve doc {i}: {e}")
    
    print("\n3️⃣ FULL-TEXT SEARCH")
    print("-" * 40)
    
    # Test different search queries
    search_queries = [
        "rust programming",
        "database",
        "meeting standup", 
        "ownership"
    ]
    
    for query in search_queries:
        try:
            results = db.query(query, limit=3)
            print(f"🔍 '{query}': {len(results.get('documents', []))} results")
            
            for doc in results.get('documents', [])[:2]:  # Show first 2
                print(f"   - {doc.get('title', 'No title')}")
        except Exception as e:
            print(f"❌ Search '{query}' failed: {e}")
    
    print("\n4️⃣ STRUCTURED QUERIES (Builder Pattern)")
    print("-" * 40)
    
    # Use QueryBuilder for type-safe queries
    try:
        results = db.query_with_builder(
            QueryBuilder()
            .text("database")
            .tag_filter("learning")
            .limit(5)
        )
        print(f"🎯 Structured query: {len(results.get('documents', []))} results")
        
        for doc in results.get('documents', []):
            tags = ", ".join(doc.get('tags', []))
            print(f"   - {doc.get('title', 'No title')} [tags: {tags}]")
            
    except Exception as e:
        print(f"❌ Structured query failed: {e}")
    
    print("\n5️⃣ DOCUMENT UPDATES")
    print("-" * 40)
    
    # Update first document
    try:
        updated_doc = db.update(sample_docs[0], {
            "content": doc['content'] + "\n\n## Updated Content\nThis document was updated via the Python client demo!",
            "tags": doc.get('tags', []) + ["updated", "demo"]
        })
        print("✅ Document updated successfully")
        
        # Verify update
        retrieved = db.get(sample_docs[0])
        print(f"📄 Updated doc has {len(retrieved['tags'])} tags")
        
    except Exception as e:
        print(f"❌ Update failed: {e}")
    
    print("\n6️⃣ PERFORMANCE TEST")
    print("-" * 40)
    
    # Quick performance test
    start_time = time.time()
    perf_docs = []
    
    for i in range(10):
        doc_id = db.insert({
            "path": f"/perf-test/doc-{i:03d}.md", 
            "title": f"Performance Test Document {i}",
            "content": f"This is performance test document number {i}. " * 10,
            "tags": ["performance", "test", f"batch-{i//5}"]
        })
        perf_docs.append(doc_id)
    
    insert_time = time.time() - start_time
    
    # Test query performance
    start_time = time.time()
    results = db.query("performance test", limit=20)
    search_time = time.time() - start_time
    
    print(f"⚡ Performance:")
    print(f"   - 10 inserts: {insert_time:.3f}s ({10/insert_time:.1f} ops/sec)")
    print(f"   - 1 search: {search_time:.3f}s ({1000*search_time:.1f}ms)")
    print(f"   - Found: {len(results.get('documents', []))} documents")
    
    print("\n7️⃣ DATABASE STATISTICS")
    print("-" * 40)
    
    try:
        final_stats = db.stats()
        print("📊 Final Statistics:")
        for key, value in final_stats.items():
            print(f"   - {key}: {value}")
    except Exception as e:
        print(f"❌ Stats failed: {e}")
    
    print("\n🎉 DEMO COMPLETE!")
    print("=" * 50)
    print("✅ All KotaDB core features demonstrated:")
    print("   - Document CRUD with builder patterns")
    print("   - Full-text search with trigram index")
    print("   - Structured queries with filters")
    print("   - Type safety and runtime validation") 
    print("   - High-performance operations")
    print("\n📚 Next steps:")
    print("   - Install client: pip install kotadb-client")
    print("   - Try examples: see examples/ directory")
    print("   - Read docs: visit documentation")
    print("   - Build your app!")

if __name__ == "__main__":
    main()