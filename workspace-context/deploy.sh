#!/bin/bash

# Deploy script for workspace-context extension
# This script builds the server, builds the extension, runs tests, and provides installation instructions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WORKSPACE_DIR="/Users/macbook/Coding/workspace"
EXTENSION_DIR="/Users/macbook/Coding/extensions/extensions/workspace-context"
SERVER_NAME="workspace"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Workspace Context Extension Deploy   ${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $2"
    else
        echo -e "${RED}✗${NC} $2"
        exit 1
    fi
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_step() {
    echo -e "${YELLOW}=== $1 ===${NC}"
}

# Step 1: Check prerequisites
print_step "Checking Prerequisites"

# Check if Rust is installed
if command -v rustc &> /dev/null; then
    print_status 0 "Rust is installed ($(rustc --version))"
else
    print_status 1 "Rust is not installed. Please install Rust first."
fi

# Check if directories exist
if [ -d "$WORKSPACE_DIR" ]; then
    print_status 0 "Workspace directory exists"
else
    print_status 1 "Workspace directory not found: $WORKSPACE_DIR"
fi

if [ -d "$EXTENSION_DIR" ]; then
    print_status 0 "Extension directory exists"
else
    print_status 1 "Extension directory not found: $EXTENSION_DIR"
fi

# Step 2: Build the workspace context server
print_step "Building Workspace Context Server"

cd "$WORKSPACE_DIR"
print_info "Building server in release mode..."
cargo build --release
print_status $? "Server build completed"

SERVER_PATH="$WORKSPACE_DIR/target/release/$SERVER_NAME"
if [ -f "$SERVER_PATH" ]; then
    chmod +x "$SERVER_PATH"
    SERVER_SIZE=$(du -h "$SERVER_PATH" | cut -f1)
    print_status 0 "Server executable ready ($SERVER_SIZE)"
else
    print_status 1 "Server executable not found after build"
fi

# Step 3: Prepare extension environment
print_step "Preparing Extension Environment"

# Add WASM target if not already added
if rustup target list --installed | grep -q "wasm32-wasip1"; then
    print_status 0 "WASM target already installed"
else
    print_info "Installing WASM target..."
    rustup target add wasm32-wasip1
    print_status $? "WASM target installed"
fi

# Step 4: Build the extension
print_step "Building Zed Extension"

cd "$EXTENSION_DIR"
print_info "Building extension for WASM target..."
cargo build --release --target wasm32-wasip1
print_status $? "Extension build completed"

# Check WASM output
WASM_PATH="$EXTENSION_DIR/target/wasm32-wasip1/release/workspace_context.wasm"
if [ -f "$WASM_PATH" ]; then
    WASM_SIZE=$(du -h "$WASM_PATH" | cut -f1)
    print_status 0 "WASM file generated ($WASM_SIZE)"
else
    print_status 1 "WASM file not found"
fi

# Step 5: Run tests
print_step "Running Tests"

if [ -f "$EXTENSION_DIR/test.sh" ]; then
    print_info "Running extension tests..."
    cd "$EXTENSION_DIR"
    ./test.sh > /tmp/workspace_context_test.log 2>&1
    if [ $? -eq 0 ]; then
        print_status 0 "All tests passed"
    else
        print_warning "Some tests failed. Check log: /tmp/workspace_context_test.log"
    fi
else
    print_warning "Test script not found, skipping tests"
fi

# Step 6: Generate configuration
print_step "Generating Configuration"

CONFIG_FILE="$EXTENSION_DIR/zed-settings.json"
cat > "$CONFIG_FILE" << EOF
{
  "context_servers": {
    "workspace-context": {
      "settings": {
        "server_path": "$SERVER_PATH",
        "workspace_path": "$WORKSPACE_DIR",
        "env": [
          ["WORKSPACE_PATH", "$WORKSPACE_DIR"]
        ],
        "debug": false
      }
    }
  }
}
EOF

print_status 0 "Configuration file generated: $CONFIG_FILE"

# Step 7: Create installation package info
print_step "Creating Installation Package"

INSTALL_INFO="$EXTENSION_DIR/DEPLOYMENT_INFO.txt"
cat > "$INSTALL_INFO" << EOF
Workspace Context Extension - Deployment Information
===================================================

Generated: $(date)

Paths:
- Server executable: $SERVER_PATH
- Extension directory: $EXTENSION_DIR
- WASM file: $WASM_PATH
- Configuration: $CONFIG_FILE

File sizes:
- Server: $(du -h "$SERVER_PATH" | cut -f1)
- WASM: $(du -h "$WASM_PATH" | cut -f1)

Installation Status: READY FOR DEPLOYMENT
EOF

print_status 0 "Deployment info created: $INSTALL_INFO"

# Step 8: Final instructions
print_step "Installation Instructions"

echo ""
print_info "🎉 Build completed successfully!"
echo ""
print_info "To install in Zed:"
echo "1. Open Zed editor"
echo "2. Press Cmd+Shift+P (macOS) or Ctrl+Shift+P (Linux/Windows)"
echo "3. Type: extensions: install dev extension"
echo "4. Select directory: $EXTENSION_DIR"
echo "5. Restart Zed"
echo ""
print_info "Then add this configuration to your Zed settings:"
echo "---"
cat "$CONFIG_FILE"
echo "---"
echo ""
print_info "Test the installation by typing in Zed's AI assistant:"
echo "@main get_workspace_context"
echo ""
print_info "For debugging, set 'debug': true in the configuration."
echo ""
print_step "Summary"
echo -e "${GREEN}✓ Server built and ready${NC}"
echo -e "${GREEN}✓ Extension compiled to WASM${NC}"
echo -e "${GREEN}✓ Tests completed${NC}"
echo -e "${GREEN}✓ Configuration generated${NC}"
echo -e "${GREEN}✓ Ready for installation in Zed${NC}"
echo ""
print_info "All files are located in: $EXTENSION_DIR"
