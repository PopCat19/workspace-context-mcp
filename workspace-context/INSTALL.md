# Installation Instructions for Workspace Context Extension

This guide will help you install and configure the Workspace Context Extension for Zed.

## Prerequisites

1. **Zed Editor**: Make sure you have Zed installed on your system
2. **Rust**: Ensure you have Rust installed with the `wasm32-wasip1` target
3. **Git**: For cloning the extensions repository

## Step 1: Prepare the Workspace Context Server

First, build the workspace context server:

```bash
cd /Users/macbook/Coding/workspace
cargo build --release
```

This will create the executable at `/Users/macbook/Coding/workspace/target/release/workspace`.

## Step 2: Build the Extension

Navigate to the extension directory and build it:

```bash
cd /Users/macbook/Coding/extensions/extensions/workspace-context
./build.sh
```

Or manually:

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

## Step 3: Install the Extension in Zed

1. Open Zed
2. Open the command palette with `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux/Windows)
3. Type `extensions: install dev extension`
4. Select the workspace-context directory: `/Users/macbook/Coding/extensions/extensions/workspace-context`
5. Restart Zed

## Step 4: Configure the Extension

Add the following configuration to your Zed settings.json file:

1. Open Zed
2. Go to `Zed > Settings` (or use `Cmd+,`)
3. Add this configuration:

```json
{
  "context_servers": {
    "workspace-context": {
      "settings": {
        "server_path": "/Users/macbook/Coding/workspace/target/release/workspace",
        "workspace_path": "/Users/macbook/Coding/workspace",
        "debug": false
      }
    }
  }
}
```

### Configuration Options

- `server_path`: **Required** - Full path to the workspace context server executable
- `workspace_path`: **Optional** - Path to the workspace to analyze (defaults to current project)
- `debug`: **Optional** - Enable debug logging (defaults to false)

## Step 5: Test the Installation

1. Open a project in Zed
2. Open the AI assistant panel
3. Try invoking the workspace context server:

```
@main get_workspace_context
```

You should see workspace information returned by the server.

## Troubleshooting

### Extension Not Loading

- Ensure the extension was built successfully for `wasm32-wasip1` target
- Check that the extension directory path is correct
- Restart Zed after installation

### Server Not Starting

- Verify the `server_path` points to the correct executable
- Make sure the executable has proper permissions (`chmod +x`)
- Check the debug logs in Zed's console

### Permission Issues

- Ensure the server executable is executable:
```bash
chmod +x /Users/macbook/Coding/workspace/target/release/workspace
```

### Debug Mode

To enable debug logging, set `"debug": true` in your configuration. This will provide more detailed logs about the server's operation.

## Updating the Extension

To update the extension:

1. Pull the latest changes from the repository
2. Rebuild the workspace context server: `cargo build --release`
3. Rebuild the extension: `./build.sh`
4. Restart Zed

## Uninstalling

To uninstall the extension:

1. Open Zed
2. Go to the Extensions panel
3. Find "Workspace Context Server" and click uninstall
4. Remove the configuration from your settings.json