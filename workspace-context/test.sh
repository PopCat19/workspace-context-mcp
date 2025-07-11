#!/bin/bash

# Test script for workspace-context extension

set -e

echo "Testing workspace-context extension..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
WORKSPACE_DIR="/Users/macbook/Coding/workspace"
EXTENSION_DIR="/Users/macbook/Coding/extensions/extensions/workspace-context"

echo "Script directory: $SCRIPT_DIR"
echo "Workspace directory: $WORKSPACE_DIR"
echo "Extension directory: $EXTENSION_DIR"

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
    echo -e "${GREEN}ℹ${NC} $1"
}

echo ""
echo "=== Pre-flight Checks ==="

# Check if Rust is installed
echo "Checking Rust installation..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    print_status 0 "Rust is installed: $RUST_VERSION"
else
    print_status 1 "Rust is not installed"
fi

# Check if wasm32-wasip1 target is available
echo "Checking WASM target..."
if rustup target list --installed | grep -q "wasm32-wasip1"; then
    print_status 0 "wasm32-wasip1 target is installed"
else
    print_warning "wasm32-wasip1 target is not installed. Installing..."
    rustup target add wasm32-wasip1
    print_status $? "Installed wasm32-wasip1 target"
fi

# Check if workspace context server exists
echo "Checking workspace context server..."
SERVER_PATH="$WORKSPACE_DIR/target/release/workspace"
if [ -f "$SERVER_PATH" ]; then
    print_status 0 "Workspace context server exists at $SERVER_PATH"

    # Check if it's executable
    if [ -x "$SERVER_PATH" ]; then
        print_status 0 "Server is executable"
    else
        print_warning "Server is not executable. Making it executable..."
        chmod +x "$SERVER_PATH"
        print_status $? "Made server executable"
    fi
else
    print_warning "Workspace context server not found. Building..."
    cd "$WORKSPACE_DIR"
    cargo build --release
    print_status $? "Built workspace context server"
fi

echo ""
echo "=== Building Extension ==="

# Build the extension
echo "Building extension..."
cd "$EXTENSION_DIR"
cargo build --release --target wasm32-wasip1
print_status $? "Extension build completed"

# Check if WASM file was generated
WASM_PATH="$EXTENSION_DIR/target/wasm32-wasip1/release/workspace_context.wasm"
if [ -f "$WASM_PATH" ]; then
    WASM_SIZE=$(du -h "$WASM_PATH" | cut -f1)
    print_status 0 "WASM file generated successfully ($WASM_SIZE)"
else
    print_status 1 "WASM file not found"
fi

echo ""
echo "=== Testing Server Functionality ==="

# Test if the server exists and is executable (basic check)
echo "Testing server basic properties..."
if [ -x "$SERVER_PATH" ]; then
    print_status 0 "Server is executable"
else
    print_status 1 "Server is not executable"
fi

# Quick file type check
FILE_TYPE=$(file "$SERVER_PATH" 2>/dev/null | head -1)
if [[ "$FILE_TYPE" == *"executable"* ]] || [[ "$FILE_TYPE" == *"Mach-O"* ]]; then
    print_status 0 "Server appears to be a valid executable"
else
    print_info "Server file type: $FILE_TYPE"
fi

echo ""
echo "=== File Structure Check ==="

# Check required files
REQUIRED_FILES=(
    "extension.toml"
    "Cargo.toml"
    "src/lib.rs"
    "README.md"
    "INSTALL.md"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$EXTENSION_DIR/$file" ]; then
        print_status 0 "$file exists"
    else
        print_status 1 "$file is missing"
    fi
done

echo ""
echo "=== Configuration Check ==="

# Check extension.toml content
echo "Checking extension.toml..."
if grep -q "workspace-context" "$EXTENSION_DIR/extension.toml"; then
    print_status 0 "Extension ID is correct"
else
    print_status 1 "Extension ID issue in extension.toml"
fi

if grep -q "context_servers.main" "$EXTENSION_DIR/extension.toml"; then
    print_status 0 "Context server configuration found"
else
    print_status 1 "Context server configuration missing"
fi

echo ""
echo "=== Installation Instructions ==="

print_info "To install this extension in Zed:"
echo "1. Open Zed"
echo "2. Press Cmd+Shift+P (macOS) or Ctrl+Shift+P (Linux/Windows)"
echo "3. Type 'extensions: install dev extension'"
echo "4. Select this directory: $EXTENSION_DIR"
echo "5. Restart Zed"
echo ""
print_info "Add this to your Zed settings.json:"
echo "{"
echo "  \"context_servers\": {"
echo "    \"workspace-context\": {"
echo "      \"settings\": {"
echo "        \"server_path\": \"$SERVER_PATH\","
echo "        \"workspace_path\": \"$WORKSPACE_DIR\","
echo "        \"debug\": false"
echo "      }"
echo "    }"
echo "  }"
echo "}"

echo ""
echo "=== Test Summary ==="
print_status 0 "All tests completed successfully!"
print_info "The workspace-context extension is ready for installation in Zed."
