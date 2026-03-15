import * as vscode from 'vscode';
import { HydraClient } from './hydraClient';
import { HydraStatusBar } from './statusBar';
import { HydraSidebarProvider } from './sidebarProvider';
import { registerCommands, handleToggleVoice } from './commands';
import { HydraTreeViewProvider } from './views/treeViewProvider';
import { HydraWebviewPanel } from './views/webviewPanel';
import { registerCodeActionsProvider } from './providers/codeActionsProvider';
import { registerCodeLensProvider } from './providers/codeLensProvider';
import { registerDiagnosticsProvider } from './providers/diagnosticsProvider';
import { registerHoverProvider } from './providers/hoverProvider';
import { registerSourceControlProvider } from './providers/sourceControlProvider';
import { registerFileDecorationProvider } from './providers/fileDecorationProvider';

let statusBar: HydraStatusBar;
let pollInterval: ReturnType<typeof setInterval> | undefined;

export function activate(context: vscode.ExtensionContext): void {
  const config = vscode.workspace.getConfiguration('hydra');
  const serverUrl = config.get<string>('serverUrl', 'http://localhost:7777');
  const showStatusBar = config.get<boolean>('showStatusBar', true);

  const client = new HydraClient(serverUrl);
  statusBar = new HydraStatusBar();

  if (!showStatusBar) {
    statusBar.dispose();
  }

  const sidebarProvider = new HydraSidebarProvider(context.extensionUri, client);

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      HydraSidebarProvider.viewType,
      sidebarProvider
    )
  );

  if (showStatusBar) {
    context.subscriptions.push(statusBar);
  }

  registerCommands(context, client, statusBar);

  // Register tree view
  const treeProvider = new HydraTreeViewProvider(client);
  vscode.window.registerTreeDataProvider('hydra-explorer', treeProvider);
  context.subscriptions.push(
    vscode.commands.registerCommand('hydra.refreshTree', () => treeProvider.refresh())
  );

  // Register webview panel command
  context.subscriptions.push(
    vscode.commands.registerCommand('hydra.showPanel', () => {
      HydraWebviewPanel.createOrShow(context.extensionUri, client);
    })
  );

  // Register voice toggle command
  context.subscriptions.push(
    vscode.commands.registerCommand('hydra.toggleVoice', () => {
      handleToggleVoice(client, statusBar);
    })
  );

  // Register language providers
  registerCodeActionsProvider(context, client);
  registerCodeLensProvider(context, client);
  registerDiagnosticsProvider(context, client);
  registerHoverProvider(context, client);

  // Register source control and file decoration providers
  registerSourceControlProvider(context, client);
  registerFileDecorationProvider(context);

  // Listen for config changes
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration('hydra.serverUrl')) {
        const newUrl = vscode.workspace
          .getConfiguration('hydra')
          .get<string>('serverUrl', 'http://localhost:7777');
        client.updateBaseUrl(newUrl);
      }
    })
  );

  // Auto-connect and periodic polling
  if (config.get<boolean>('autoConnect', true)) {
    checkConnection(client, sidebarProvider);
  }

  pollInterval = setInterval(() => {
    checkConnection(client, sidebarProvider);
  }, 10000);

  context.subscriptions.push({
    dispose: () => {
      if (pollInterval) {
        clearInterval(pollInterval);
      }
    },
  });
}

async function checkConnection(
  client: HydraClient,
  sidebar: HydraSidebarProvider
): Promise<void> {
  try {
    const running = await client.isServerRunning();
    if (running) {
      const status = await client.getStatus();

      // Feed live stats to status bar (spec: Pattern 19 + Hydra Pattern 5)
      statusBar.updateStats({
        sistersConnected: status.sistersConnected || 0,
        sistersTotal: status.sistersTotal || 17,
        profileName: status.activeProfile || '',
        beliefsLoaded: status.beliefsLoaded || 0,
        sessionCost: status.sessionCost || 0,
        compiledPatterns: status.compiledPatterns || 0,
        compiledSavings: status.compiledSavings || 0,
      });

      if (status.pendingApprovals > 0) {
        statusBar.update('approvalNeeded', `${status.pendingApprovals} pending`);
      } else if (status.activeRuns > 0) {
        // Show thinking verb during active runs
        const verb = status.thinkingVerb || 'Working';
        statusBar.update('working', verb);
      } else {
        statusBar.update('idle');
      }

      // Push briefing to webview if available
      if (status.briefingItems && HydraWebviewPanel.currentPanel) {
        HydraWebviewPanel.currentPanel.postBriefing(status.briefingItems);
      }

      await sidebar.refreshData();
    } else {
      statusBar.update('offline');
    }
  } catch {
    statusBar.update('offline');
  }
}

export function deactivate(): void {
  if (pollInterval) {
    clearInterval(pollInterval);
  }
}
