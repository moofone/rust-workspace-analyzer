# Memgraph Database Setup

This document contains instructions for setting up Memgraph as the graph database backend for the Rust Workspace Analyzer.

## Installation

### Using Docker (Recommended)

Start Memgraph with the platform image that includes Memgraph Lab:

```bash
docker run -d \
  --name memgraph-rust-analyzer \
  -p 7687:7687 \
  -p 7444:7444 \
  -p 3000:3000 \
  memgraph/memgraph-platform:latest
```

Ports:
- `7687`: Bolt protocol for database connections
- `7444`: HTTP server for monitoring
- `3000`: Memgraph Lab web interface

### Verify Installation

Check that Memgraph is running:

```bash
docker ps | grep memgraph
```

Access Memgraph Lab at `http://localhost:3000` to verify the database is accessible.

## Schema Initialization

The analyzer creates its schema automatically on first run. However, you can manually run setup commands if needed.

### Manual Schema Setup Commands

Run these commands in Memgraph Lab after creating a fresh database:

### 1. Clear any existing data
```cypher
MATCH (n) DETACH DELETE n;
```

### 2. Create uniqueness constraints
```cypher
-- Ensure unique nodes based on their IDs
CREATE CONSTRAINT ON (t:Type) ASSERT t.id IS UNIQUE;
CREATE CONSTRAINT ON (f:Function) ASSERT f.id IS UNIQUE;
CREATE CONSTRAINT ON (m:Module) ASSERT m.id IS UNIQUE;
CREATE CONSTRAINT ON (c:Crate) ASSERT c.name IS UNIQUE;
```

### 3. Create indexes for performance
```cypher
-- High-cardinality indexes for efficient lookups
CREATE INDEX ON :Type(name);
CREATE INDEX ON :Type(crate);
CREATE INDEX ON :Function(name);
CREATE INDEX ON :Function(crate);
CREATE INDEX ON :Module(name);
CREATE INDEX ON :Actor(is_distributed);
CREATE INDEX ON :Actor(is_test);
```

### 4. Schema Validation Queries

After running the analyzer, use these queries to validate the schema:

#### Check Actor node properties:
```cypher
MATCH (a:Type:Actor) 
RETURN keys(a) as properties
LIMIT 1;
```

Expected properties should include:
- `name`, `crate`, `file`, `is_test`, `is_distributed`, `actor_type`, `distributed_messages`

## Node Types and Properties

### Type Nodes (`:Type`)
All Rust types (structs, enums, traits, etc.) with these properties:
- `id`: Unique identifier
- `name`: Type name  
- `qualified_name`: Fully qualified name
- `crate`: Crate name
- `module`: Module path
- `file`: File path
- `line_start`, `line_end`: Source location
- `kind`: One of "Struct", "Enum", "Trait", "TypeAlias", "Union"
- `visibility`: "pub", "pub(crate)", "private", etc.
- `is_generic`: Boolean for generic types
- `is_test`: Boolean for test-related types
- `doc_comment`: Documentation string
- `fields`: JSON array of struct fields
- `variants`: JSON array of enum variants  
- `methods`: JSON array of method names
- `embedding_text`: Text for semantic search

### Actor Nodes (`:Type:Actor`)
Types that implement the Actor trait, with additional properties:
- All `:Type` properties plus:
- `is_distributed`: Boolean for distributed actors
- `actor_type`: "Local", "Distributed", or "Unknown"
- `distributed_messages`: JSON array of distributed message types (for distributed actors via distributed_actor! macro)
- `local_messages`: JSON array of local message types (for all actors via impl Message<T> for ActorType)

#### Check for distributed actors:
```cypher
MATCH (a:Type:Actor) 
WHERE a.is_distributed = true
RETURN a.name, a.crate, a.distributed_messages
ORDER BY a.name;
```

#### Check all actors with their message handling:
```cypher
MATCH (a:Type:Actor) 
RETURN a.name, a.crate, a.file, a.is_distributed, 
       a.distributed_messages, a.local_messages
ORDER BY a.name;
```

#### Check for test actors:
```cypher
MATCH (a:Type:Actor) 
WHERE a.is_test = true
RETURN a.name, a.crate, a.file
ORDER BY a.name;
```

#### Verify node counts:
```cypher
MATCH (n) 
RETURN labels(n) as node_type, count(*) as count
ORDER BY count DESC;
```

Expected node types:
- `Type` (includes actors)
- `Function` 
- `Module`
- `Crate`
- `MessageType`

#### Verify relationship counts:
```cypher
MATCH ()-[r]->()
RETURN type(r) as relationship_type, count(*) as count
ORDER BY count DESC;
```

Expected relationships:
- `CALLS` (function calls)
- `SPAWNS` (actor spawning)
- `HANDLES` (message handling)
- `SENDS` (message sending)

## Troubleshooting

### Issue: Missing properties on Actor nodes
If Actor nodes are missing `is_distributed`, `distributed_messages`, or other properties:

1. Check that the analyzer is running with the latest code
2. Verify the configuration points to the correct Memgraph instance
3. Ensure `clean_start = true` in config.toml for a fresh run

### Issue: No distributed actors found
```cypher
-- Check if any actors exist at all
MATCH (a:Type:Actor) RETURN count(*) as total_actors;

-- Check for distributed_actor! macro usage in logs
-- Look for "distributed_actor!" in the analyzer output
```

### Issue: Duplicate nodes
If you see duplicate actors:
1. Ensure constraints are properly created
2. Run with `clean_start = true`
3. Check for connection pool issues in logs

## Schema Migration for local_messages Field

If you have an existing database without the `local_messages` property, run this migration query:

```cypher
-- Add local_messages property to existing Actor nodes
MATCH (a:Type:Actor)
WHERE a.local_messages IS NULL
SET a.local_messages = [];
```

## Database Connection

Ensure your `config.toml` has the correct connection settings:
```toml
[memgraph]
uri = "bolt://memgraph.rust-workspace-analyzer.orb.local:7687"
username = ""
password = ""
clean_start = true
batch_size = 1000
```