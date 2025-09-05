use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio;

use workspace_analyzer::{Config, mcp::EnhancedMcpServer};

#[derive(Parser)]
#[command(name = "workspace-analyzer")]
#[command(about = "Tree-sitter based Rust workspace analyzer with Memgraph 3.0 GraphRAG")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start MCP server")]
    McpServer {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
    },
    #[command(about = "Analyze workspace")]
    Analyze {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        #[arg(long)]
        output_json: Option<PathBuf>,
        #[arg(long, help = "Populate Memgraph with analysis results")]
        populate_graph: bool,
    },
    #[command(about = "Check architecture violations")]
    CheckArchitecture {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
    },
    #[command(about = "Analyze impact of changes to a specific symbol")]
    ImpactAnalysis {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
        #[arg(short, long, help = "Symbol to analyze (e.g., 'MyStruct', 'my_function', 'MyTrait')")]
        symbol: String,
        #[arg(long, help = "Symbol type: function, struct, trait, enum, type")]
        symbol_type: Option<String>,
    },
    #[command(about = "Health check Memgraph connection")]
    HealthCheck {
        #[arg(short, long, default_value = "config.toml")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::McpServer { config } => {
            eprintln!("🚀 Starting Enhanced MCP Server with Tree-sitter + Memgraph 3.0");
            start_mcp_server(config).await
        }
        Commands::Analyze { config, output_json, populate_graph } => {
            eprintln!("🔍 Analyzing workspace");
            analyze_workspace(config, output_json, populate_graph).await
        }
        Commands::CheckArchitecture { config } => {
            eprintln!("🏗️ Checking architecture violations");
            check_architecture(config).await
        }
        Commands::ImpactAnalysis { config, symbol, symbol_type } => {
            eprintln!("🎯 Analyzing impact of changes to symbol: {}", symbol);
            analyze_symbol_impact(config, symbol, symbol_type).await
        }
        Commands::HealthCheck { config } => {
            eprintln!("🏥 Checking Memgraph connection");
            health_check(config).await
        }
    }
}

async fn start_mcp_server(config_path: PathBuf) -> Result<()> {
    let server = EnhancedMcpServer::new(config_path.to_str().unwrap()).await?;
    
    eprintln!("📡 MCP Server ready - send JSON-RPC requests via stdin");
    eprintln!("💡 Available methods: initialize, workspace_context, analyze_change_impact, check_architecture_violations, semantic_search");
    
    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let stdout = tokio::io::stdout();
    let mut writer = tokio::io::BufWriter::new(stdout);

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(request_json) = serde_json::from_str::<serde_json::Value>(&line) {
                    let request = workspace_analyzer::mcp::McpRequest {
                        id: request_json.get("id").cloned(),
                        method: request_json.get("method")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        params: request_json.get("params").cloned(),
                    };
                    
                    let response = server.handle_request(request).await;
                    let response_json = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": response.id,
                        "result": response.result,
                        "error": response.error
                    });
                    
                    writer.write_all(response_json.to_string().as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

async fn analyze_workspace(config_path: PathBuf, output_json: Option<PathBuf>, populate_graph: bool) -> Result<()> {
    let config = Config::from_file(config_path.to_str().unwrap())?;
    let mut analyzer = workspace_analyzer::WorkspaceAnalyzer::new_with_config(config.clone())?;
    
    let snapshot = if populate_graph {
        eprintln!("🔍 Analyzing workspace and populating Memgraph...");
        let graph = workspace_analyzer::MemgraphClient::new(&config).await?;
        
        // First populate the graph with analysis
        let _ = analyzer.analyze_and_populate_graph(
            Some(&graph), 
            None,  // embedding_gen
            None,  // architecture
            None,  // semantic_search  
            None   // incremental_updater
        ).await?;
        
        eprintln!("✅ Graph population complete, creating snapshot...");
        // Then create snapshot for the rest of the function
        analyzer.create_snapshot().await?
    } else {
        eprintln!("🔍 Creating workspace snapshot with unified analyzer...");
        analyzer.create_snapshot().await?
    };
    
    eprintln!("📦 Found {} crates ({} workspace members)", 
        snapshot.crates.len(), 
        snapshot.crates.iter().filter(|c| c.is_workspace_member).count());

    eprintln!("🌳 Parsed with tree-sitter:");
    for crate_meta in &snapshot.crates {
        if crate_meta.is_workspace_member {
            if let Some(symbols) = snapshot.symbols.get(&crate_meta.name) {
                eprintln!("  ✅ {} - {} functions, {} types", 
                    crate_meta.name, 
                    symbols.functions.len(),
                    symbols.types.len());
            }
        }
    }
    
    eprintln!("🔗 References already resolved by analyzer...");
    
    eprintln!("📊 Analysis complete!");
    println!("Total Functions: {}", snapshot.functions.len());
    println!("Total Types: {}", snapshot.types.len());
    // Note: Cross-crate call calculation would need to be added to WorkspaceSnapshot
    
    if let Some(output_path) = output_json {
        let analysis_result = serde_json::json!({
            "crates": snapshot.crates,
            "functions": snapshot.functions.len(),
            "types": snapshot.types.len(),
            "actors": snapshot.actors.len(),
            "distributed_actors": snapshot.distributed_actors.len(),
        });
        
        std::fs::write(&output_path, serde_json::to_string_pretty(&analysis_result)?)?;
        eprintln!("💾 Results written to {:?}", output_path);
    }
    
    Ok(())
}

async fn check_architecture(config_path: PathBuf) -> Result<()> {
    let config = Config::from_file(config_path.to_str().unwrap())?;
    let graph = workspace_analyzer::MemgraphClient::new(&config).await?;
    let analyzer = workspace_analyzer::ArchitectureAnalyzer::new(std::sync::Arc::new(graph), config);
    
    eprintln!("🏗️ Analyzing architecture...");
    let report = analyzer.analyze_architecture().await?;
    
    println!("Architecture Analysis Report");
    println!("===========================");
    println!("Total Violations: {}", report.summary.total_violations);
    println!("Errors: {}", report.summary.error_count);
    println!("Warnings: {}", report.summary.warning_count);
    
    if !report.violations.is_empty() {
        println!("\nViolations:");
        for violation in report.violations.iter().take(10) {
            println!("  {} - {} -> {} ({}:{})", 
                match violation.severity {
                    workspace_analyzer::architecture::ViolationSeverity::Error => "❌",
                    workspace_analyzer::architecture::ViolationSeverity::Warning => "⚠️ ",
                    workspace_analyzer::architecture::ViolationSeverity::Info => "ℹ️ ",
                },
                violation.from_crate, 
                violation.to_crate, 
                violation.file, 
                violation.line);
        }
        
        if report.violations.len() > 10 {
            println!("  ... and {} more", report.violations.len() - 10);
        }
    }
    
    Ok(())
}

async fn health_check(config_path: PathBuf) -> Result<()> {
    let config = Config::from_file(config_path.to_str().unwrap())?;
    
    eprintln!("🔗 Connecting to Memgraph at {}...", config.memgraph.uri);
    let graph = workspace_analyzer::MemgraphClient::new(&config).await?;
    
    let is_healthy = graph.health_check().await?;
    
    if is_healthy {
        println!("✅ Memgraph 3.0 connection is healthy");
    } else {
        println!("⚠️  Memgraph connection is slow (>50ms response time)");
    }
    
    let stats = graph.get_statistics().await?;
    println!("Graph Statistics:");
    println!("  Crate nodes: {}", stats.crate_nodes);
    println!("  Function nodes: {}", stats.function_nodes);
    println!("  Type nodes: {}", stats.type_nodes);
    println!("  Call edges: {}", stats.call_edges);
    
    Ok(())
}


async fn analyze_symbol_impact(config_path: PathBuf, symbol: String, symbol_type: Option<String>) -> Result<()> {
    println!("🎯 Symbol Impact Analysis");
    println!("═══════════════════════════");
    
    // Load configuration
    let config = Config::from_file(config_path.to_str().unwrap())?;
    
    // Create analyzer
    let mut analyzer = workspace_analyzer::WorkspaceAnalyzer::new_with_config(config.clone())?;
    let graph_client = workspace_analyzer::graph::MemgraphClient::new(&config).await?;
    
    // Create workspace snapshot
    println!("🔍 Creating workspace snapshot...");
    let snapshot = analyzer.create_snapshot().await?;
    println!("  Found {} crates ({} workspace members)", 
        snapshot.crates.len(),
        snapshot.crates.iter().filter(|c| c.is_workspace_member).count()
    );
    
    // Search for the symbol across all crates
    println!("🔎 Searching for symbol '{}' in workspace...", symbol);
    
    let mut found_symbols = Vec::new();
    
    // Search for the symbol in functions
    for func in &snapshot.functions {
        if func.name == symbol || func.qualified_name.contains(&symbol) {
            found_symbols.push(format!("Function: {} in {}:{}", 
                func.qualified_name, func.file_path, func.line_start));
        }
    }
    
    // Search for the symbol in types
    for typ in &snapshot.types {
        if typ.name == symbol || typ.qualified_name.contains(&symbol) {
            found_symbols.push(format!("{:?}: {} in {}:{}", 
                typ.kind, typ.qualified_name, typ.file_path, typ.line_start));
        }
    }
    
    if found_symbols.is_empty() {
        println!("❌ Symbol '{}' not found in the workspace", symbol);
        println!("💡 Try searching for:");
        println!("   - Function names (e.g., 'calculate_price')");
        println!("   - Struct names (e.g., 'Order')");  
        println!("   - Trait names (e.g., 'Serialize')");
        println!("   - Enum names (e.g., 'OrderStatus')");
        return Ok(());
    }
    
    println!("✅ Found {} matches for symbol '{}':", found_symbols.len(), symbol);
    for (i, found_symbol) in found_symbols.iter().enumerate() {
        println!("  {}. {}", i + 1, found_symbol);
    }
    
    // Analyze impact by looking at function references
    println!("\n📊 Impact Analysis:");
    println!("──────────────────");
    
    // Count direct usages using function references
    let mut direct_usages = 0;
    let mut calling_functions = Vec::new();
    
    // Analyze function references to find usages
    for (target, callers) in &snapshot.function_references {
        if target.contains(&symbol) {
            direct_usages += callers.len();
            calling_functions.extend(callers.clone());
        }
    }
    
    if direct_usages > 0 {
        println!("📞 Direct function calls: {}", direct_usages);
        println!("🔗 Called by {} different functions", calling_functions.len());
        
        // Show sample callers
        let sample_size = std::cmp::min(5, calling_functions.len());
        if sample_size > 0 {
            println!("📋 Sample callers:");
            for (i, caller) in calling_functions.iter().take(sample_size).enumerate() {
                println!("  {}. {}", i + 1, caller);
            }
            if calling_functions.len() > sample_size {
                println!("  ... and {} more", calling_functions.len() - sample_size);
            }
        }
    } else {
        println!("📞 No direct function calls found");
    }
    
    // Analyze by symbol type
    if let Some(sym_type) = symbol_type {
        println!("\n🔍 Type-specific analysis for '{}':", sym_type);
        match sym_type.to_lowercase().as_str() {
            "struct" => {
                println!("  • Look for field access patterns");
                println!("  • Check for trait implementations");
                println!("  • Verify constructor usage");
            },
            "trait" => {
                println!("  • Check for implementations across crates");
                println!("  • Look for trait bounds in generics");
                println!("  • Verify method calls on trait objects");
            },
            "enum" => {
                println!("  • Check variant usage patterns");
                println!("  • Look for match expressions");
                println!("  • Verify serialization/deserialization");
            },
            "function" => {
                println!("  • Direct function calls (shown above)");
                println!("  • Function pointer usage");
                println!("  • Higher-order function usage");
            },
            _ => {
                println!("  • General symbol analysis performed");
            }
        }
    }
    
    // Provide change impact guidance
    println!("\n💡 Change Impact Guidance:");
    println!("─────────────────────────");
    
    if direct_usages == 0 {
        println!("✅ LOW IMPACT: Symbol appears to have no direct dependencies");
        println!("   • Safe to modify implementation");
        println!("   • Consider if it's unused and can be removed");
    } else if direct_usages < 5 {
        println!("⚠️  MEDIUM IMPACT: Symbol has {} direct usages", direct_usages);
        println!("   • Review each caller before making changes");
        println!("   • Consider backward compatibility");
        println!("   • Update tests for affected functionality");
    } else {
        println!("🚨 HIGH IMPACT: Symbol has {} direct usages", direct_usages);
        println!("   • Breaking changes will affect many parts of codebase");
        println!("   • Consider deprecation strategy");
        println!("   • Extensive testing required");
        println!("   • Document migration path for users");
    }
    
    println!("\n📝 Recommendations:");
    println!("   • Run tests after any changes");
    println!("   • Check for compiler warnings");
    println!("   • Review documentation that might reference this symbol");
    if direct_usages > 0 {
        println!("   • Consider using `cargo check` to validate changes");
        println!("   • Use IDE 'Find All References' for deeper analysis");
    }
    
    Ok(())
}
