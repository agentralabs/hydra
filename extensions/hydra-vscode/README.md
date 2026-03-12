# Hydra VS Code Extension

Hydra AI Agent Orchestration for VS Code. Connects to a running Hydra server to manage agent runs, approvals, and sister status directly from your editor.

## Features

- **Status bar indicator** with live connection state (idle, working, approval needed, offline)
- **Sidebar panel** with run input, active runs, pending approvals, and sister health
- **Commands** for running intents, stopping runs, approving/denying actions, and checking status
- **Keyboard shortcuts** for quick access

## Requirements

A running Hydra server (default: `http://localhost:7777`). The extension degrades gracefully when the server is unavailable.

## Installation

```bash
cd extensions/hydra-vscode
npm install
npm run compile
```

To install in VS Code, use `code --install-extension` with the packaged `.vsix`, or symlink this directory into `~/.vscode/extensions/`.

## Commands

| Command | Shortcut | Description |
|---|---|---|
| Hydra: Run Intent | `Cmd+Shift+H` | Start a new Hydra run with a natural language intent |
| Hydra: Stop All | `Cmd+Shift+K` | Stop all running Hydra runs |
| Hydra: Show Status | | Display server status in the output channel |
| Hydra: Approve Pending | | Pick and approve a pending action |
| Hydra: Deny Pending | | Pick and deny a pending action |
| Hydra: Sister Status | | Show sister connection status |
| Hydra: Toggle Sidebar | | Open the Hydra sidebar |

## Settings

| Setting | Default | Description |
|---|---|---|
| `hydra.serverUrl` | `http://localhost:7777` | Hydra server URL |
| `hydra.autoConnect` | `true` | Auto-connect on startup |
| `hydra.showStatusBar` | `true` | Show status bar indicator |
