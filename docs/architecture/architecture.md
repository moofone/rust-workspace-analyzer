# Rust Workspace Analyzer - System Architecture

## Table of Contents
1. [Project Overview](#project-overview)
2. [System Architecture](#system-architecture)
3. [Core Components](#core-components)
4. [Analysis Pipeline](#analysis-pipeline)
5. [Data Models](#data-models)
6. [Integration Points](#integration-points)
7. [Performance & Scalability](#performance--scalability)
8. [Deployment Architecture](#deployment-architecture)

## Project Overview

### Purpose
The Rust Workspace Analyzer is a sophisticated tool that provides deep architectural analysis and insights for Rust workspaces. It's designed for seamless integration with Claude Code through the MCP (Model Context Protocol), enabling AI-assisted code analysis and architectural guidance.

### Key Capabilities
- **Cross-Crate Function Analysis**: Accurate tracking of function calls across workspace boundaries
- **Architecture Violation Detection**: Identifies layer violations and circular dependencies
- **Test Coverage Intelligence**: Finds heavily-used functions without test coverage
- **Change Impact Analysis**: Predicts the effects of code modifications
- **Real-time Integration**: Provides analysis through MCP protocol for Claude Code

### Technology Stack
- **Core Language**: Rust 2024 edition
- **AST Parsing**: tree-sitter with tree-sitter-rust
- **Graph Database**: Memgraph (via neo4rs)
- **Protocol**: MCP (Model Context Protocol) over JSON-RPC
- **Concurrency**: tokio async runtime with rayon for CPU-bound tasks
- **Workspace Analysis**: cargo_metadata for Rust workspace introspection

## System Architecture

The system follows a layered architecture with four distinct tiers:

```
┌─────────────────────────────────────────────────────────────┐
│                    Interface Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ MCP Server  │  │ CLI Binary  │  │ Test Utilities      │ │
│  │ (stdio)     │  │ Interface   │  │ & Debug Tools       │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                   Analysis Engine Layer                     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │            WorkspaceAnalyzer                            │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │ │
│  │  │ Symbol      │  │ Reference   │  │ Architecture    │ │ │
│  │  │ Extraction  │  │ Resolution  │  │ Validation      │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                   Data Storage Layer                        │
│  ┌─────────────┐           ┌─────────────────────────────┐   │
│  │ In-Memory   │           │      Memgraph Database     │   │
│  │ Registries  │◄─────────►│   (Optional Enhancement)    │   │
│  │ & Caches    │           │                             │   │
│  └─────────────┘           └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────┐
│                  AST Parsing Layer                          │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                 tree-sitter                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │ │
│  │  │ Rust        │  │ File        │  │ Symbol          │ │ │
│  │  │ Grammar     │  │ Walking     │  │ Recognition     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

1. **Interface Layer**: Handles external communication and user interaction
2. **Analysis Engine Layer**: Core business logic for workspace analysis
3. **Data Storage Layer**: Manages analysis results and provides fast lookup
4. **AST Parsing Layer**: Low-level code parsing and symbol extraction

## Core Components

### WorkspaceAnalyzer (`src/analyzer/workspace.rs`)

The heart of the system, responsible for analyzing Rust workspaces through a two-pass approach.

```rust
pub struct WorkspaceAnalyzer {
    root_path: PathBuf,
    parser: Parser,
    packages: Vec<Package>,
}
```

**Key Responsibilities:**
- **Pass 1**: Symbol extraction - builds function and type registries
- **Pass 2**: Reference resolution - analyzes function calls and dependencies
- **Workspace Discovery**: Uses `cargo_metadata` to understand crate structure
- **AST Parsing**: Leverages tree-sitter for precise Rust code analysis

**Core Methods:**
- `analyze_workspace()` → `WorkspaceSnapshot`: Main analysis entry point
- `extract_functions_from_file()`: Symbol extraction per file
- `resolve_references()`: Cross-crate reference resolution

### MCP Server (`src/mcp/server.rs`)

Implements the Model Context Protocol for Claude Code integration.

```rust
pub struct WorkspaceMcpServer {
    analyzer: Arc<RwLock<WorkspaceAnalyzer>>,
    graph_client: Arc<RwLock<MemgraphClient>>,
    workspace_root: std::path::PathBuf,
    current_snapshot: Arc<RwLock<Option<WorkspaceSnapshot>>>,
}
```

**Protocol Methods:**
- `workspace_context`: Provides comprehensive workspace overview
- `analyze_test_coverage`: Identifies heavily-used untested functions
- `check_architecture_violations`: Detects layer violations
- `find_dependency_issues`: Finds circular dependencies
- `analyze_change_impact`: Predicts impact of code changes
- `suggest_safe_refactoring`: Recommends safe refactoring opportunities
- `validate_proposed_change`: Validates changes for safety

### MemgraphClient (`src/graph/memgraph_client.rs`)

Optional graph database integration for complex dependency queries.

```rust
pub struct MemgraphClient {
    pub graph: Graph,
    pub workspace_name: String,
}
```

**Features:**
- **Performance Schema**: Optimized indexes and constraints for fast queries
- **Health Monitoring**: Connection health checks with performance metrics
- **Workspace Isolation**: Separate analysis data per workspace

**Schema Design:**
```cypher
// Nodes
(:Function {workspace, qualified_name, name, module})
(:Type {workspace, qualified_name, name, module})

// Relationships
(:Function)-[:CALLS]->(:Function)
(:Function)-[:DEPENDS_ON]->(:Type)
```

### Function Registry (`src/analyzer/workspace.rs:74-78`)

In-memory registry providing fast symbol lookup and resolution.

```rust
pub struct FunctionRegistry {
    pub functions_by_name: HashMap<String, Vec<String>>,
    pub functions_by_qualified: HashMap<String, RustFunction>,
    pub public_functions: HashSet<String>,
}
```

**Optimization Features:**
- **Multi-index Lookup**: Name-based and qualified-name-based access
- **Visibility Tracking**: Separates public vs private functions
- **Cross-Crate Resolution**: Handles complex module path resolution

### Binary Interfaces (`src/bin/`)

Multiple entry points serving different use cases:

- **`mcp_server_stdio.rs`**: Main MCP server for Claude Code integration
- **`test_architecture.rs`**: Architecture validation testing
- **`check_memgraph.rs`**: Database connectivity testing
- **`mcp_client.rs`**: MCP protocol testing and debugging

## Analysis Pipeline

### Two-Pass Analysis Workflow

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Discovery     │───►│  Pass 1: Symbol │───►│ Pass 2: Reference│
│   Phase         │    │   Extraction    │    │   Resolution     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ • Find Cargo.toml│    │ • Parse AST     │    │ • Resolve calls │
│ • Load metadata │    │ • Extract funcs │    │ • Build graph   │
│ • List packages │    │ • Extract types │    │ • Detect cycles │
│ • Map file paths│    │ • Build registry│    │ • Validate arch │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Detailed Phase Breakdown

#### Discovery Phase
1. **Workspace Detection**: Locates `Cargo.toml` and determines workspace root
2. **Metadata Loading**: Uses `cargo_metadata` to understand crate structure
3. **File Enumeration**: Walks directory tree to find all `.rs` files
4. **Package Mapping**: Maps files to their containing crates

#### Pass 1: Symbol Extraction
```rust
// Parallel processing per file
files.par_iter()
    .map(|file| self.extract_functions_from_file(file))
    .collect()
```

1. **AST Parsing**: tree-sitter parses each Rust file into syntax tree
2. **Function Detection**: Identifies function declarations with signatures
3. **Type Detection**: Extracts struct, enum, trait, and type alias definitions
4. **Module Resolution**: Builds qualified names based on module hierarchy
5. **Registry Building**: Populates in-memory lookup structures

#### Pass 2: Reference Resolution
```rust
// Cross-crate reference resolution
for reference in function_calls {
    let resolved = self.resolve_function_call(reference, &registry)?;
    dependency_graph.add_edge(resolved);
}
```

1. **Call Site Analysis**: Identifies function call expressions
2. **Import Resolution**: Tracks `use` statements and module imports
3. **Qualified Name Resolution**: Resolves `crate::module::function` calls
4. **Cross-Crate Detection**: Identifies calls crossing crate boundaries
5. **Graph Construction**: Builds dependency relationships

### Performance Optimizations

- **Parallel File Processing**: Uses `rayon` for concurrent file analysis
- **Smart Caching**: Caches AST trees and analysis results
- **Incremental Analysis**: Only re-analyzes changed files (future feature)
- **Memory Pooling**: Reuses parser instances across files

## Data Models

### Core Domain Objects

#### RustFunction (`src/analyzer/workspace.rs:10-20`)
```rust
#[derive(Debug, Clone)]
pub struct RustFunction {
    pub name: String,              // Function name (e.g., "new")
    pub qualified_name: String,    // Full path (e.g., "crate::module::Type::new")
    pub file_path: PathBuf,        // Source file location
    pub line_start: usize,         // Starting line number
    pub line_end: usize,          // Ending line number
    pub module: String,           // Containing module
    pub visibility: String,       // pub, pub(crate), private
    pub parameters: Vec<String>,  // Parameter signatures
    pub return_type: Option<String>, // Return type if specified
}
```

#### RustType (`src/analyzer/workspace.rs:22-32`)
```rust
#[derive(Debug, Clone)]
pub struct RustType {
    pub name: String,              // Type name
    pub qualified_name: String,    // Full qualified path
    pub file_path: PathBuf,        // Source location
    pub line_start: usize,         // Line range
    pub line_end: usize,
    pub module: String,           // Containing module
    pub visibility: String,       // Visibility modifier
    pub type_kind: String,        // struct, enum, trait, type_alias
}
```

#### FunctionReference (`src/analyzer/workspace.rs:44-52`)
```rust
#[derive(Debug, Clone)]
pub struct FunctionReference {
    pub target_function: String,   // Qualified name of called function
    pub calling_function: String,  // Qualified name of caller
    pub call_type: CallType,       // How the function was called
    pub cross_crate: bool,         // Cross-crate boundary flag
    pub from_test: bool,          // Called from test code
    pub file_path: PathBuf,       // Location of call site
    pub line: usize,              // Line number of call
}
```

#### CallType Taxonomy (`src/analyzer/workspace.rs:55-62`)
```rust
#[derive(Debug, Clone)]
pub enum CallType {
    Direct,      // function_name()
    Method,      // obj.method_name()
    Qualified,   // crate::module::function_name()
    Import,      // use crate::func; func()
    Macro,       // macro_name!()
    TraitImpl,   // impl trait::Trait for Type
}
```

### Analysis Results

#### WorkspaceSnapshot
```rust
pub struct WorkspaceSnapshot {
    pub functions: Vec<RustFunction>,
    pub types: Vec<RustType>,
    pub function_references: Vec<FunctionReference>,
    pub dependencies: Vec<Dependency>,
    pub analysis_stats: AnalysisStats,
    pub timestamp: SystemTime,
}
```

**Immutable Design**: Snapshots provide point-in-time views of workspace state, enabling safe concurrent access and historical comparison.

### Graph Database Schema

When Memgraph is enabled, the system maintains a graph representation:

```cypher
// Function nodes
CREATE (f:Function {
    workspace: $workspace_name,
    qualified_name: $qualified_name,
    name: $name,
    module: $module,
    visibility: $visibility,
    file_path: $file_path,
    line_start: $line_start,
    line_end: $line_end
})

// Function call relationships
CREATE (caller:Function)-[:CALLS {
    call_type: $call_type,
    cross_crate: $cross_crate,
    from_test: $from_test,
    file_path: $file_path,
    line: $line
}]->(target:Function)
```

## Integration Points

### MCP Protocol Implementation

The system implements MCP (Model Context Protocol) over JSON-RPC for seamless Claude Code integration.

#### Protocol Flow
```
Claude Code ←→ JSON-RPC ←→ MCP Server ←→ WorkspaceAnalyzer
                (stdio)
```

#### Available Tools

| Tool | Purpose | Key Data |
|------|---------|----------|
| `workspace_context` | Overview of workspace structure | Function/type counts, crate list |
| `analyze_test_coverage` | Find untested high-usage functions | Reference counts, test presence |
| `check_architecture_violations` | Detect layer violations | Dependency direction, layer jumping |
| `find_dependency_issues` | Identify circular dependencies | Dependency cycles, problematic paths |
| `analyze_change_impact` | Predict change effects | Dependent functions, blast radius |
| `suggest_safe_refactoring` | Recommend improvements | Safe refactoring opportunities |
| `validate_proposed_change` | Safety validation | Architecture compliance |

#### Example Tool Response Format
```json
{
  "id": 1,
  "result": {
    "summary": "Workspace analysis completed",
    "functions": 5173,
    "types": 2035,
    "cross_crate_calls": 12399,
    "analysis": {
      "heavily_used_untested": [
        {
          "function": "trading_config::config::get_config",
          "references": 70,
          "cross_crate_refs": 68,
          "location": "src/config.rs:42"
        }
      ]
    }
  }
}
```

### Claude Code Integration Patterns

#### Proactive Analysis
Claude Code can automatically invoke analysis tools based on context:
- Code modifications trigger change impact analysis
- Architecture reviews invoke violation checking
- Test planning uses coverage analysis

#### Interactive Queries
Users can ask natural language questions that map to specific tools:
- "What functions need tests?" → `analyze_test_coverage`
- "Are there any architecture problems?" → `check_architecture_violations`
- "What would break if I change X?" → `analyze_change_impact`

### Configuration and Environment

#### Environment Variables
- `RUST_ANALYZER_CONFIG`: Path to configuration file
- `MEMGRAPH_URI`: Graph database connection string
- `RUST_LOG`: Logging verbosity control

#### Claude Code MCP Configuration
```json
{
  "mcpServers": {
    "rust-workspace-analyzer": {
      "command": "/usr/local/bin/rust-workspace-analyzer",
      "args": ["--workspace", "."],
      "env": {
        "RUST_LOG": "info",
        "MEMGRAPH_URI": "bolt://localhost:7687"
      }
    }
  }
}
```

## Performance & Scalability

### Performance Characteristics

#### Analysis Performance
- **Small Workspaces** (<100 files): ~500ms analysis time
- **Medium Workspaces** (100-500 files): ~2-5s analysis time  
- **Large Workspaces** (500+ files): ~10-30s analysis time

#### Memory Usage
- **Base Memory**: ~50MB for analyzer core
- **Per-File Overhead**: ~1MB per analyzed file
- **Graph Database**: Additional ~100MB for Memgraph connection

### Scalability Strategies

#### Parallel Processing
```rust
use rayon::prelude::*;

// Parallel file processing
files.par_iter()
    .map(|file| analyze_file(file))
    .collect::<Result<Vec<_>>>()?
```

- **File-Level Parallelism**: Each file analyzed on separate thread
- **CPU Utilization**: Scales with available CPU cores
- **Memory Management**: Controlled per-thread memory usage

#### Caching Strategy
```rust
pub struct AnalysisCache {
    file_hashes: HashMap<PathBuf, u64>,
    function_cache: HashMap<PathBuf, Vec<RustFunction>>,
    reference_cache: HashMap<PathBuf, Vec<FunctionReference>>,
}
```

- **File Hash Tracking**: Only re-analyze modified files
- **Result Caching**: Cache parsed functions and references
- **Incremental Updates**: Future feature for real-time analysis

#### Graph Database Optimization
```cypher
// Performance indexes
CREATE INDEX function_name_idx FOR (f:Function) ON (f.name);
CREATE INDEX function_module_idx FOR (f:Function) ON (f.module);

// Unique constraints  
CREATE CONSTRAINT workspace_function_unique 
ON (f:Function) ASSERT (f.workspace, f.qualified_name) IS UNIQUE;
```

- **Strategic Indexing**: Optimized for common query patterns
- **Connection Pooling**: Reuse database connections
- **Batch Operations**: Bulk inserts for better throughput

### Resource Management

#### Memory Optimization
- **Lazy Loading**: Load analysis results on demand
- **Reference Counting**: Share common string data
- **Parser Reuse**: Maintain parser instance pool

#### Disk I/O Optimization  
- **Batch File Reading**: Read multiple files in single I/O operations
- **mmap Usage**: Memory-mapped file access for large files
- **SSD Optimization**: Optimized for modern storage patterns

## Deployment Architecture

### Local Development Setup
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Claude Code   │───►│ MCP Server      │───►│ Rust Workspace │
│   (Main App)    │    │ (Local Binary)  │    │ (Target Code)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │ Memgraph DB     │
                       │ (Docker)        │
                       └─────────────────┘
```

### Production Integration
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   IDE/Editor    │───►│ Language Server │───►│ Workspace       │
│   Integration   │    │ Mode            │    │ Analysis        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │ Analysis Cache  │
                       │ & Results Store │
                       └─────────────────┘
```

### Fault Tolerance

#### Graceful Degradation
- **Memgraph Optional**: System works without graph database
- **Partial Analysis**: Continues analysis even if some files fail
- **Error Recovery**: Robust error handling with detailed diagnostics

#### Error Handling Strategy
```rust
pub enum AnalysisError {
    ParseError { file: PathBuf, details: String },
    ResolutionError { symbol: String, context: String },
    DatabaseError { operation: String, cause: String },
}
```

### Monitoring and Observability

#### Logging Framework
- **Structured Logging**: JSON-formatted logs for analysis
- **Performance Metrics**: Analysis timing and resource usage
- **Error Tracking**: Detailed error context and stack traces

#### Health Checks
- **Database Connectivity**: Memgraph health monitoring
- **Analysis Performance**: Timeout detection and recovery
- **Resource Usage**: Memory and CPU monitoring

---

*This architecture supports the Rust Workspace Analyzer's mission of providing deep, accurate, and performant analysis of Rust codebases through seamless Claude Code integration.*