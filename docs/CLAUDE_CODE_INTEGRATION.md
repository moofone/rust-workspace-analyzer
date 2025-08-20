# Claude Code Integration Guide

## Quick Setup

### 1. Install the Analyzer
```bash
# Clone and build
git clone https://github.com/anthropics/rust-workspace-analyzer
cd rust-workspace-analyzer
./install.sh
```

### 2. Configure Claude Code
Add the MCP server using Claude Code's built-in command:

```bash
claude mcp add rust-workspace-analyzer -- /Users/greg/Dev/git/rust-workspace-analyzer/target/release/mcp-server-stdio -w /Users/greg/Dev/git/trading-backend-poc
```

### 3. Verify Configuration
Check that the MCP server is properly configured:
```
/mcp
```

You should see `rust-workspace-analyzer` listed as an available MCP server.

## 🎯 Example Conversations

### Getting Started
```
You: "Analyze this Rust workspace"

Claude: I'll analyze your Rust workspace to provide insights about its architecture, dependencies, and structure.

[Claude uses workspace_context tool and provides a comprehensive overview including:
- Total functions and types discovered
- Crate structure and cross-crate calls
- Sample functions and types from each crate
- Overall workspace metrics]

Example output:
## 📊 Overview
- **Workspace**: /Users/greg/Dev/git/trading-backend-poc
- **Total Crates**: 11
- **Total Functions**: 5173
- **Total Types**: 2035
- **Function References**: 55811
- **Cross-crate Calls**: 12399

## 🔧 Sample Functions (showing 20 of 5173):
- `trading_core::config::get_config` in trading-core
- `trading_exchanges::binance::connect` in trading-exchanges
```

### Test Coverage Analysis
```
You: "Show me which functions need tests"

Claude: I'll analyze your test coverage to identify heavily-used functions that lack tests.

[Claude uses analyze_test_coverage tool and shows:
- Functions with many references but no tests  
- Cross-crate usage patterns
- Priority recommendations for test coverage
- Coverage statistics by crate]

Example output:
🚨 Priority: Untested High-Usage Functions
- **get_config** in `trading_core::config` (70 refs, 68 cross-crate) - `src/config.rs:45`
- **init_config** in `trading_core::init` (16 refs, 16 cross-crate) - `src/init.rs:12`
- **cluster_simple** in `trading_data_services::clustering` (14 refs) - `src/clustering.rs:234`

📈 Coverage by Crate
- **trading-core**: 45% coverage (23/51 functions, 8 high-usage untested)
- **trading-exchanges**: 12% coverage (5/42 functions, 15 high-usage untested)
```

### Architecture Validation
```
You: "Check for architecture problems"

Claude: I'll check your workspace for architectural violations and dependency issues.

[Claude uses check_architecture_violations tool and finds:
- Layer dependency violations (lower layers calling higher layers)
- Circular dependencies between modules
- Deep import chains that could use preludes
- Cross-layer jumps that skip intermediate layers]

Example findings:
🚨 Layer Violations (3):
⚠️ **Upward Dependency**: `trading-core` (layer 0) → `trading-runtime` (layer 5) - get_config() calls runtime_init() in src/config.rs:45

🔀 Cross-Layer Jumps (12):
🔀 **Layer Jump**: `trading-core` → `trading-exchanges` (jumps 3 layers) - connect() calls binance_api() in src/exchange.rs:123

📦 Consider Prelude Usage (25):
📦 **Deep Import**: `trading_exchanges::binance::api::rest::public` (depth 5) - consider using prelude in src/binance.rs:67
```

### Change Impact Analysis
```
You: "What would happen if I change the OrderManager interface?"

Claude: I'll analyze the impact of changing OrderManager across your workspace.

[Claude uses analyze_change_impact tool and shows:
- All functions that match OrderManager
- Functions that depend on OrderManager  
- Estimated blast radius of the change
- Files and line numbers affected]

Example output:
## Matching Functions (2):
- `trading_core::order::OrderManager::new` in /src/order.rs
- `trading_core::order::OrderManager::execute` in /src/order.rs

## Potential Impact (23 dependents):
- use calls from trading_exchanges/src/binance.rs (line 45)
- call calls from trading_strategy/src/momentum.rs (line 127)
... and 21 more
```

### Dependency Analysis
```
You: "Show me all dependencies of the authentication module"

Claude: I'll map out all the dependencies for your authentication module.

[Claude analyzes dependencies and shows:
- Direct dependencies
- Transitive dependencies  
- Reverse dependencies (what depends on auth)
- Potential circular dependency risks]
```

## 🛠️ Available Tools Reference

| Command Examples | Tool Used | What It Does |
|------------------|-----------|--------------|
| "Analyze this workspace" | `workspace_context` | Complete workspace overview with function/type counts |
| "Show test coverage" | `analyze_test_coverage` | Find heavily-used untested functions with cross-crate analysis |
| "Check architecture" | `check_architecture_violations` | Find layer violations, circular deps, deep imports |
| "Find circular dependencies" | `find_dependency_issues` | Detect circular dependency cycles |
| "Impact of changing X" | `analyze_change_impact` | Predict change effects and dependent functions |
| "Suggest refactoring" | `suggest_safe_refactoring` | Safe improvement suggestions (🚧 in development) |
| "Validate this change" | `validate_proposed_change` | Check change safety (🚧 in development) |

## 💡 Best Practices

### Start with Overview
Begin conversations with broad analysis:
```
"Give me an overview of this Rust workspace"
"What's the overall architecture?"
```

### Focus on Problems
Identify issues early:
```
"What are the main problems in this codebase?"
"Show me architecture violations"
"Find untested critical functions"
```

### Plan Changes Safely
Before making changes:
```
"What would break if I modify the PaymentProcessor?"
"Is it safe to refactor the database layer?"
"Show me all dependencies of this module"
```

### Monitor Test Coverage
Keep track of testing needs:
```
"Which functions should I write tests for first?"
"Show me test coverage by importance"
"What's our overall test coverage?"
```

## 🎯 Power User Tips

### Combine Multiple Analyses
```
You: "I'm planning to refactor the trading engine. Give me a complete analysis of what I need to consider."

Claude will automatically use multiple tools:
1. workspace_context - to understand the trading engine's role
2. analyze_change_impact - to see what depends on it
3. check_architecture_violations - to find current issues
4. analyze_test_coverage - to ensure adequate testing
```

### Focus on Specific Areas
```
"Focus your analysis on the payment processing crate"
"Only show me issues in the data layer"
"Analyze just the public APIs"
```

### Get Actionable Recommendations
```
"What should I fix first in this codebase?"
"Prioritize these issues by impact"
"Give me a refactoring roadmap"
```

## 🚨 Troubleshooting

### "No analysis available"
- Ensure you're in a Rust workspace (has Cargo.toml)
- Check MCP server status in Claude Code
- Try: "Initialize workspace analysis"

### Slow analysis
- First analysis of large workspaces takes time
- Subsequent analyses are cached and fast
- Consider excluding large generated files

### Memgraph connection issues
```bash
# Check if Memgraph is running
docker ps | grep memgraph

# Start if needed
docker run -d --name memgraph-rust-analyzer -p 7687:7687 memgraph/memgraph-platform
```

### Debug mode
If issues persist, enable debug logging:
```bash
RUST_LOG=debug rust-workspace-analyzer --workspace /path/to/project
```

## 🎬 Demo Workflow

Here's a complete workflow for analyzing a new Rust workspace:

### 1. Initial Overview
```
You: "I just inherited this Rust codebase. Help me understand it."
Claude: [Provides workspace overview with key metrics and structure]
```

### 2. Find Problems
```
You: "What problems should I be aware of?"
Claude: [Runs architecture checks and finds violations, circular deps, etc.]
```

### 3. Prioritize Testing
```
You: "What functions need tests most urgently?"
Claude: [Shows heavily-used untested functions ranked by impact]
```

### 4. Plan Improvements
```
You: "Create a roadmap for improving this codebase"
Claude: [Combines all analyses to suggest prioritized improvements]
```

### 5. Validate Changes
```
You: "I want to refactor the user authentication. Is this safe?"
Claude: [Analyzes change impact and suggests safe refactoring approach]
```

## ✨ Advanced Features

### Cross-Crate Analysis
The analyzer excels at multi-crate workspaces:
- Tracks function calls across crate boundaries
- Identifies cross-crate architectural violations
- Maps complex dependency relationships

### Real-Time Feedback
- Fast analysis suitable for interactive development
- Cached results for quick follow-up questions
- Incremental updates as code changes

### Rich Context Integration
Claude Code gets deep workspace context:
- Function usage patterns
- Architectural relationships
- Test coverage gaps
- Refactoring opportunities

---

**🚀 Ready to supercharge your Rust development with AI-powered architecture insights!**