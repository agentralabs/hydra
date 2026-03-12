"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const hydraClient_1 = require("./hydraClient");
const statusBar_1 = require("./statusBar");
const sidebarProvider_1 = require("./sidebarProvider");
const commands_1 = require("./commands");
const treeViewProvider_1 = require("./views/treeViewProvider");
const webviewPanel_1 = require("./views/webviewPanel");
const codeActionsProvider_1 = require("./providers/codeActionsProvider");
const codeLensProvider_1 = require("./providers/codeLensProvider");
const diagnosticsProvider_1 = require("./providers/diagnosticsProvider");
const hoverProvider_1 = require("./providers/hoverProvider");
let statusBar;
let pollInterval;
function activate(context) {
    const config = vscode.workspace.getConfiguration('hydra');
    const serverUrl = config.get('serverUrl', 'http://localhost:7777');
    const showStatusBar = config.get('showStatusBar', true);
    const client = new hydraClient_1.HydraClient(serverUrl);
    statusBar = new statusBar_1.HydraStatusBar();
    if (!showStatusBar) {
        statusBar.dispose();
    }
    const sidebarProvider = new sidebarProvider_1.HydraSidebarProvider(context.extensionUri, client);
    context.subscriptions.push(vscode.window.registerWebviewViewProvider(sidebarProvider_1.HydraSidebarProvider.viewType, sidebarProvider));
    if (showStatusBar) {
        context.subscriptions.push(statusBar);
    }
    (0, commands_1.registerCommands)(context, client, statusBar);
    // Register tree view
    const treeProvider = new treeViewProvider_1.HydraTreeViewProvider(client);
    vscode.window.registerTreeDataProvider('hydra-explorer', treeProvider);
    context.subscriptions.push(vscode.commands.registerCommand('hydra.refreshTree', () => treeProvider.refresh()));
    // Register webview panel command
    context.subscriptions.push(vscode.commands.registerCommand('hydra.showPanel', () => {
        webviewPanel_1.HydraWebviewPanel.createOrShow(context.extensionUri, client);
    }));
    // Register voice toggle command
    context.subscriptions.push(vscode.commands.registerCommand('hydra.toggleVoice', () => {
        (0, commands_1.handleToggleVoice)(client, statusBar);
    }));
    // Register language providers
    (0, codeActionsProvider_1.registerCodeActionsProvider)(context, client);
    (0, codeLensProvider_1.registerCodeLensProvider)(context, client);
    (0, diagnosticsProvider_1.registerDiagnosticsProvider)(context, client);
    (0, hoverProvider_1.registerHoverProvider)(context, client);
    // Listen for config changes
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('hydra.serverUrl')) {
            const newUrl = vscode.workspace
                .getConfiguration('hydra')
                .get('serverUrl', 'http://localhost:7777');
            client.updateBaseUrl(newUrl);
        }
    }));
    // Auto-connect and periodic polling
    if (config.get('autoConnect', true)) {
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
async function checkConnection(client, sidebar) {
    try {
        const running = await client.isServerRunning();
        if (running) {
            const status = await client.getStatus();
            if (status.pendingApprovals > 0) {
                statusBar.update('approvalNeeded', `${status.pendingApprovals} pending`);
            }
            else if (status.activeRuns > 0) {
                statusBar.update('working', `${status.activeRuns} running`);
            }
            else {
                statusBar.update('idle');
            }
            await sidebar.refreshData();
        }
        else {
            statusBar.update('offline');
        }
    }
    catch {
        statusBar.update('offline');
    }
}
function deactivate() {
    if (pollInterval) {
        clearInterval(pollInterval);
    }
}
//# sourceMappingURL=extension.js.map