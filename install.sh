#!/bin/bash

# Rust Workspace Analyzer Installation Script
# For use with Claude Code MCP

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/rust-workspace-analyzer"

echo "🦀 Installing Rust Workspace Analyzer for Claude Code..."

# Create directories
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"

# Check for required dependencies
echo "📋 Checking dependencies..."

# Check if Docker is available for Memgraph
if ! command -v docker &> /dev/null; then
    echo "⚠️  Docker not found. Memgraph requires Docker to run."
    echo "   Please install Docker from https://www.docker.com/get-started"
    echo "   You can continue without Memgraph (limited functionality)"
    read -p "   Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo "✅ Docker found"
    
    # Start Memgraph container if not running
    if ! docker ps | grep -q "memgraph-rust-analyzer"; then
        echo "🗄️  Starting Memgraph database..."
        docker run -d \
            --name memgraph-rust-analyzer \
            -p 7687:7687 \
            -p 7444:7444 \
            -p 3000:3000 \
            --memory=1g \
            memgraph/memgraph-platform:latest \
            || echo "⚠️  Failed to start Memgraph (may already be running)"
    else
        echo "✅ Memgraph already running"
    fi
fi

# Copy binary
echo "📦 Installing binary..."
if [ -f "$SCRIPT_DIR/target/release/mcp-server-stdio" ]; then
    cp "$SCRIPT_DIR/target/release/mcp-server-stdio" "$INSTALL_DIR/rust-workspace-analyzer"
    chmod +x "$INSTALL_DIR/rust-workspace-analyzer"
    echo "✅ Binary installed to $INSTALL_DIR/rust-workspace-analyzer"
else
    echo "❌ Release binary not found. Please run: cargo build --release --bin mcp-server-stdio"
    exit 1
fi

# Create config file
echo "⚙️  Creating configuration..."
cat > "$CONFIG_DIR/config.json" << EOF
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
        "file": "$CONFIG_DIR/rust-analyzer.log"
    }
}
EOF

# Create Claude Code MCP configuration
echo "🤖 Creating Claude Code MCP configuration..."
CLAUDE_CONFIG_DIR="$HOME/.config/claude-code"
mkdir -p "$CLAUDE_CONFIG_DIR"

# Check if mcp_settings.json exists
MCP_CONFIG="$CLAUDE_CONFIG_DIR/mcp_settings.json"
if [ -f "$MCP_CONFIG" ]; then
    echo "📝 Adding to existing Claude Code MCP configuration..."
    # Create backup
    cp "$MCP_CONFIG" "$MCP_CONFIG.backup.$(date +%s)"
    
    # Add our server to existing config (simplified - user may need to merge manually)
    echo "⚠️  Please manually add the following to your Claude Code MCP configuration:"
    echo
    cat << EOF
{
  "mcpServers": {
    "rust-workspace-analyzer": {
      "command": "$INSTALL_DIR/rust-workspace-analyzer",
      "args": [],
      "env": {
        "RUST_ANALYZER_CONFIG": "$CONFIG_DIR/config.json"
      }
    }
  }
}
EOF
    echo
else
    echo "📝 Creating new Claude Code MCP configuration..."
    cat > "$MCP_CONFIG" << EOF
{
  "mcpServers": {
    "rust-workspace-analyzer": {
      "command": "$INSTALL_DIR/rust-workspace-analyzer",
      "args": [],
      "env": {
        "RUST_ANALYZER_CONFIG": "$CONFIG_DIR/config.json"
      }
    }
  }
}
EOF
fi

# Add to PATH if needed
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "📍 Adding $INSTALL_DIR to PATH..."
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc" 2>/dev/null || true
    echo "   Please restart your shell or run: export PATH=\"$HOME/.local/bin:\$PATH\""
fi

# Test installation
echo "🧪 Testing installation..."
if "$INSTALL_DIR/rust-workspace-analyzer" --version 2>/dev/null; then
    echo "✅ Installation successful!"
else
    echo "⚠️  Binary installed but may need dependencies"
fi

echo
echo "🎉 Installation complete!"
echo
echo "📚 Next steps:"
echo "   1. Restart Claude Code to load the new MCP server"
echo "   2. In a Rust workspace, try: 'Analyze this workspace architecture'"
echo "   3. Or: 'Show me test coverage analysis'"
echo
echo "📁 Files installed:"
echo "   Binary: $INSTALL_DIR/rust-workspace-analyzer"
echo "   Config: $CONFIG_DIR/config.json"
echo "   Claude Code MCP: $MCP_CONFIG"
echo
echo "🗄️  Memgraph database running at: bolt://localhost:7687"
echo "   Web interface: http://localhost:3000"
echo
echo "📋 To uninstall:"
echo "   rm -f $INSTALL_DIR/rust-workspace-analyzer"
echo "   rm -rf $CONFIG_DIR"
echo "   docker stop memgraph-rust-analyzer && docker rm memgraph-rust-analyzer"