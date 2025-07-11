# Workspace Context Server

A Model Context Protocol (MCP) server that provides workspace context information to AI assistants in Zed.

## Features

- Extracts workspace structure and content
- Provides file and directory information
- Configurable workspace paths
- Debug mode support

## Installation

### Prerequisites

- [Zed Editor](https://zed.dev/) installed
- [Rust](https://rustup.rs/) with `wasm32-wasip1` target
- Git (for cloning)

### Step 1: Build the Workspace Context Server

Clone and build the workspace context server:

```bash
git clone <repository-url>
cd workspace
cargo build --release
```

The executable will be created at `target/release/workspace`.

### Step 2: Build the Zed Extension

Navigate to the extension directory and build:

```bash
cd workspace-context
./build.sh
```

Or manually:

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

### Step 3: Install Extension in Zed

1. Open Zed
2. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux/Windows)
3. Type `extensions: install dev extension`
4. Select the `workspace-context` directory
5. Restart Zed

### Step 4: Configure in Zed Settings

Add to your Zed `settings.json`:

```json
{
  "context_servers": {
    "workspace-context": {
      "settings": {
        "server_path": "/path/to/workspace/target/release/workspace",
        "workspace_path": "/path/to/your/project",
        "debug": false
      }
    }
  }
}
```

Replace `/path/to/workspace` with your actual workspace path.

## Settings

- `server_path`: Path to the workspace context server executable (required)
- `workspace_path`: Path to the workspace to analyze (optional, defaults to current project)
- `debug`: Enable debug logging (optional, defaults to false)

## Usage

Once installed and configured, you can use the workspace context server in Zed's AI assistant:

```
@workspace-context get_workspace_context
```

This provides the AI with comprehensive information about your workspace structure and content.

## Troubleshooting

**Extension not loading:**
- Ensure the extension was built for `wasm32-wasip1` target
- Restart Zed after installation

**Server not starting:**
- Verify `server_path` points to the correct executable
- Make the executable file executable: `chmod +x /path/to/workspace/target/release/workspace`
- Check debug logs by setting `"debug": true`
