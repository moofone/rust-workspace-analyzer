# Rust Workspace Analyzer

**Internal Use Only** - Experimental semantic codebase analysis for LLM integration.

📖 **For detailed architecture information, see [docs/architecture.md](docs/architecture.md)**

## Features

- **Hybrid Analysis Engine**: Combines tree-sitter syntax parsing with rust-analyzer LSP for semantic accuracy
- **Cross-Crate Dependency Analysis**: Tracks function calls and dependencies across workspace boundaries  
- **Architecture Violation Detection**: Identifies layer violations and circular dependencies
- **Test Coverage Analysis**: Finds heavily-used functions without test coverage
- **Change Impact Analysis**: Predicts impact of code modifications
- **MCP Integration**: Works with Claude Code through Model Context Protocol

**Note**: See [docs/MEMGRAPH_SETUP.md](docs/MEMGRAPH_SETUP.md) for detailed database setup instructions.
