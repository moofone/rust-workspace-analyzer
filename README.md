# Rust Workspace Analyzer for Trading Backend PoC

A specialized MCP (Model Context Protocol) server that provides deep architectural analysis and insights for the trading-backend-poc Rust workspace, designed for seamless integration with Claude Code.

## 🚀 Features

### 📊 Comprehensive Workspace Analysis
- **Function Reference Tracking**: Accurately counts function calls across crate boundaries
- **Cross-Crate Dependency Analysis**: Identifies dependencies between workspace crates
- **Architecture Violation Detection**: Finds layer violations and circular dependencies
- **Test Coverage Analysis**: Identifies heavily-used functions without tests

### 🏗️ Architecture Insights
- **Layer Dependency Validation**: Enforces architectural layers (core → data → strategy → runtime)
- **Circular Dependency Detection**: Finds problematic dependency cycles
- **Prelude Usage Suggestions**: Recommends when to use prelude modules for deep imports
- **Change Impact Analysis**: Predicts the impact of modifying specific functions or types

### 🎯 Test Coverage Intelligence
- **Heavy Usage Detection**: Finds functions with many references but no tests
- **Cross-Crate Reference Counting**: Accurate function call tracking across workspace boundaries
- **Test Priority Ranking**: Prioritizes functions needing test coverage by usage metrics
- **Coverage Statistics**: Per-crate coverage percentages and untested function counts

## 📦 Installation

### Prerequisites
- **Rust**: Latest stable version
- **Docker**: For Memgraph database (optional but recommended)
- **Claude Code**: For MCP integration

### Quick Install

```bash
# Clone and build
git clone https://github.com/anthropics/rust-workspace-analyzer
cd rust-workspace-analyzer
cargo build --release --bin mcp-server-stdio

# Run installation script
./install.sh
```

**Note**: This analyzer is specifically designed for the trading-backend-poc project architecture and layer structure.

### Manual Installation

1. **Build the release binary**:
   ```bash
   cargo build --release --bin mcp-server-stdio
   ```

2. **Copy to local bin**:
   ```bash
   cp target/release/mcp-server-stdio ~/.local/bin/rust-workspace-analyzer
   chmod +x ~/.local/bin/rust-workspace-analyzer
   ```

3. **Start Memgraph** (optional):
   ```bash
   docker run -d \
     --name memgraph-rust-analyzer \
     -p 7687:7687 \
     -p 7444:7444 \
     -p 3000:3000 \
     memgraph/memgraph-platform:latest
   ```

4. **Configure Claude Code MCP**:
   Add to `~/.config/claude-code/mcp_settings.json`:
   ```json
   {
     "mcpServers": {
       "rust-workspace-analyzer": {
         "command": "/home/user/.local/bin/rust-workspace-analyzer",
         "args": [],
         "env": {}
       }
     }
   }
   ```

## 🔧 Usage

### With Claude Code

Once installed, restart Claude Code and try these commands in the trading-backend-poc workspace:

#### Architecture Analysis
```
"Analyze the architecture of this workspace"
"Show me any architecture violations"
"Find circular dependencies"
```

#### Test Coverage Analysis
```
"Show me test coverage analysis"
"Which functions are heavily used but untested?"
"What's the test coverage by crate?"
```

#### Change Impact Analysis
```
"What would happen if I change the PositionManager interface?"
"Analyze the impact of modifying handle_order function"
```

#### Dependency Analysis
```
"Show me the dependency structure"
"Are there any layer violations?"
"What functions depend on DatabaseManager?"
```

### Standalone Usage

You can also run the analyzer directly:

```bash
# Start MCP server for current directory
rust-workspace-analyzer

# Analyze specific workspace
rust-workspace-analyzer --workspace /path/to/rust/project

# Show version
rust-workspace-analyzer --version
```

## 📊 Example Output

### Test Coverage Analysis
```
# Test Coverage Analysis

## 📊 Coverage Summary
- **Total Functions**: 5172
- **Test Functions**: 1588  
- **Heavily Used & Untested**: 200 ⚠️
- **Heavily Used & Tested**: 6 ✅

## 🚨 Priority: Untested High-Usage Functions
- **get_config** in `trading-config` (70 refs, 68 cross-crate)
- **init_config** in `trading-config` (16 refs, 16 cross-crate)
- **cluster_simple** in `zone_utils` (14 refs, 0 cross-crate)

## 📈 Coverage by Crate
- **trading-config**: 19% coverage (4/21 functions, 17 high-usage untested)
- **trading-core**: 51% coverage (84/162 functions, 78 high-usage untested)
```

### Architecture Violations
```
# Architecture Violations

## 🏗️ Layer Dependency Analysis
- ⚠️  **Upward Dependency**: `trading-runtime` (layer 5) → `trading-core` (layer 1)
- 🔀 **Layer Jump**: `trading-runtime` → `trading-data-services` (jumps 3 layers)

## 📦 Consider Prelude Usage (513):
- **Deep Import**: `trading_exchanges::crypto::futures::binance::BinanceFuturesActor` (depth 4)
```

## 🛠️ Available Tools

The MCP server provides these tools to Claude Code:

| Tool | Description |
|------|-------------|
| `workspace_context` | Get comprehensive workspace overview |
| `analyze_test_coverage` | Find heavily-used untested functions |
| `check_architecture_violations` | Detect layer violations and architectural issues |
| `find_dependency_issues` | Find circular dependencies |
| `analyze_change_impact` | Predict impact of code changes |
| `suggest_safe_refactoring` | Suggest safe refactoring opportunities |
| `validate_proposed_change` | Validate changes for safety |

## ⚙️ Configuration

### Environment Variables
- `RUST_ANALYZER_CONFIG`: Path to configuration file
- `MEMGRAPH_URI`: Memgraph connection string (default: `bolt://localhost:7687`)
- `RUST_LOG`: Logging level (default: `info`)

### Configuration File
Create `~/.config/rust-workspace-analyzer/config.json`:
```json
{
  "memgraph": {
    "uri": "bolt://localhost:7687",
    "username": "",
    "password": "",
    "enabled": true
  },
  "analysis": {
    "max_file_size": 1000000,
    "skip_test_files": false,
    "cache_results": true
  },
  "logging": {
    "level": "info",
    "file": "~/.config/rust-workspace-analyzer/analyzer.log"
  }
}
```

## 🏗️ Architecture

The analyzer uses a two-pass analysis approach:

1. **Pass 1**: Build function registry and extract all symbols
2. **Pass 2**: Resolve function references and build dependency graph

### Key Components
- **WorkspaceAnalyzer**: Core analysis engine using tree-sitter
- **FunctionRegistry**: Fast function lookup and resolution
- **MemgraphClient**: Graph database for complex dependency queries
- **MCP Server**: Protocol implementation for Claude Code integration

## 🤝 Integration with Claude Code

The analyzer integrates seamlessly with Claude Code through the MCP protocol:

1. **Auto-Discovery**: Claude Code automatically discovers available tools
2. **Context-Aware**: Provides workspace context for better code assistance
3. **Real-Time Analysis**: Fast analysis suitable for interactive use
4. **Rich Responses**: Structured markdown responses with actionable insights

## 📝 Examples

### Find Heavily Used Untested Functions
Ask Claude Code: *"Show me functions that are called frequently but don't have tests"*

The analyzer will:
1. Count all function references across the workspace
2. Identify functions without corresponding tests
3. Rank by usage frequency and cross-crate impact
4. Provide file locations and reference counts

### Architecture Validation  
Ask Claude Code: *"Check if this workspace follows good architectural patterns"*

The analyzer will:
1. Validate layer dependencies (core → data → strategy → runtime)
2. Find circular dependencies
3. Suggest prelude usage for deep imports
4. Report any violations with specific file locations

### Change Impact Analysis
Ask Claude Code: *"What would break if I change the OrderManager interface?"*

The analyzer will:
1. Find all references to OrderManager
2. Identify dependent functions and types
3. Calculate blast radius of the change
4. Suggest safe refactoring approaches

## 🔍 Troubleshooting

### Common Issues

#### "No workspace analysis available"
- Ensure you're in a Rust workspace (has `Cargo.toml`)
- Check that the MCP server initialized successfully
- Look for errors in Claude Code's MCP logs

#### "Memgraph connection failed"  
- Ensure Docker is running
- Start Memgraph: `docker run -d -p 7687:7687 memgraph/memgraph-platform`
- Check connection with: `docker ps | grep memgraph`

#### "Analysis too slow"
- Large workspaces (>1000 files) may take time on first analysis
- Results are cached for subsequent runs
- Consider excluding large generated files

### Debug Mode
Run with debug logging:
```bash
RUST_LOG=debug rust-workspace-analyzer --workspace /path/to/project
```

### Log Files
- MCP server logs: `~/.config/rust-workspace-analyzer/analyzer.log`
- Claude Code MCP logs: Check Claude Code settings

## 🤖 Claude Code Integration Guide

### Example Conversations

#### Getting Started
```
You: "Analyze this Rust workspace"
Claude: [Uses workspace_context tool to provide overview]

You: "What functions need tests?"
Claude: [Uses analyze_test_coverage tool to show priority functions]

You: "Check for architecture problems"  
Claude: [Uses check_architecture_violations tool to find issues]
```

#### Deep Analysis
```
You: "I'm refactoring the payment system. What should I be careful about?"
Claude: [Uses analyze_change_impact with payment-related functions]

You: "Show me all the dependencies of the OrderProcessor"
Claude: [Uses dependency analysis to map all connections]
```

### Best Practices

1. **Start Broad**: Begin with workspace context to understand the codebase
2. **Focus on Issues**: Use architecture violation checks to find problems
3. **Prioritize Testing**: Use test coverage analysis to guide testing efforts
4. **Validate Changes**: Use change impact analysis before major refactoring

## 📚 Technical Details

### Function Reference Counting

The analyzer implements accurate cross-crate function reference counting through:

- **Tree-sitter Parsing**: Precise AST analysis of Rust code
- **Import Resolution**: Tracks use statements and qualified calls  
- **Cross-Crate Detection**: Identifies calls across crate boundaries
- **Test Context Filtering**: Separates test calls from production usage

### Performance Optimizations

- **Incremental Analysis**: Only re-analyzes changed files
- **Parallel Processing**: Uses rayon for multi-threaded file processing
- **Graph Database**: Memgraph for complex dependency queries
- **Smart Caching**: Caches results between analysis runs

## 📋 Requirements

### System Requirements
- **Memory**: 2GB+ for large workspaces
- **CPU**: Multi-core recommended for parallel processing
- **Disk**: 1GB for Memgraph database and caches

### Rust Version Support
- **Minimum**: Rust 1.70+
- **Recommended**: Latest stable
- **Edition Support**: 2018, 2021

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

### Development Setup
```bash
git clone https://github.com/anthropics/rust-workspace-analyzer
cd rust-workspace-analyzer
cargo build
cargo test
```

**Important**: This tool is tailored for the trading-backend-poc project's specific architecture patterns and may not work correctly with other Rust projects.

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Issues**: GitHub Issues for bugs and feature requests
- **Discussions**: GitHub Discussions for questions
- **Documentation**: This README and inline code documentation

---

**Ready to analyze your Rust workspace? Install now and integrate with Claude Code for powerful architectural insights!** 🦀✨