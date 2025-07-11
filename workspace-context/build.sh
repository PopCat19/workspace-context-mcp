#!/bin/bash

# Build script for workspace-context extension

set -e

echo "Building workspace-context extension..."

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
WORKSPACE_DIR="$(dirname "$SCRIPT_DIR")"

echo "Script directory: $SCRIPT_DIR"
echo "Workspace directory: $WORKSPACE_DIR"

# Build the main workspace context server
echo "Building workspace context server..."
cd "$WORKSPACE_DIR"
cargo build --release

# Check if the executable was built successfully
if [ ! -f "$WORKSPACE_DIR/target/release/workspace" ]; then
    echo "Error: workspace executable not found at $WORKSPACE_DIR/target/release/workspace"
    exit 1
fi

echo "Workspace context server built successfully at: $WORKSPACE_DIR/target/release/workspace"

# Build the Zed extension
echo "Building Zed extension..."
cd "$SCRIPT_DIR"

# Add WASM target if not already added
rustup target add wasm32-wasip1

# Build for WASM target
cargo build --release --target wasm32-wasip1

echo "Extension built successfully!"
echo ""
echo "To install this extension in Zed:"
echo "1. Open Zed"
echo "2. Open the command palette (Cmd+Shift+P)"
echo "3. Type 'extensions: install dev extension'"
echo "4. Select this directory: $SCRIPT_DIR"
echo "5. Restart Zed"
echo ""
echo "Then add this to your Zed settings.json:"
echo "{"
echo "  \"context_servers\": {"
echo "    \"workspace-context\": {"
echo "      \"settings\": {"
echo "        \"server_path\": \"$WORKSPACE_DIR/target/release/workspace\","
echo "        \"workspace_path\": \"$WORKSPACE_DIR\","
echo "        \"debug\": false"
echo "      }"
echo "    }"
echo "  }"
echo "}"
