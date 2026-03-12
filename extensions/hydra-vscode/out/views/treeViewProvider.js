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
exports.HydraTreeViewProvider = void 0;
const vscode = __importStar(require("vscode"));
class HydraTreeViewProvider {
    constructor(client) {
        this.client = client;
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh() {
        this._onDidChangeTreeData.fire(undefined);
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (!element) {
            // Root items
            return [
                new HydraTreeItem('Status', vscode.TreeItemCollapsibleState.Expanded, 'status'),
                new HydraTreeItem('Active Runs', vscode.TreeItemCollapsibleState.Expanded, 'runs'),
                new HydraTreeItem('Sisters', vscode.TreeItemCollapsibleState.Collapsed, 'sisters'),
                new HydraTreeItem('Recent', vscode.TreeItemCollapsibleState.Collapsed, 'recent'),
            ];
        }
        switch (element.contextValue) {
            case 'status':
                return this.getStatusItems();
            case 'runs':
                return this.getRunItems();
            case 'sisters':
                return this.getSisterItems();
            case 'recent':
                return this.getRecentItems();
            default:
                return [];
        }
    }
    async getStatusItems() {
        const connected = await this.client.isServerRunning();
        const item = new HydraTreeItem(connected ? 'Connected' : 'Disconnected', vscode.TreeItemCollapsibleState.None, 'status-item');
        item.iconPath = new vscode.ThemeIcon(connected ? 'circle-filled' : 'circle-outline');
        return [item];
    }
    async getRunItems() {
        const running = await this.client.isServerRunning();
        if (!running) {
            return [new HydraTreeItem('No active runs', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
        try {
            const status = await this.client.rpc('status', {});
            const runs = status?.active_runs || [];
            if (runs.length === 0) {
                return [new HydraTreeItem('No active runs', vscode.TreeItemCollapsibleState.None, 'empty')];
            }
            return runs.map((r) => {
                const item = new HydraTreeItem(r.intent || 'Run', vscode.TreeItemCollapsibleState.None, 'run');
                item.iconPath = new vscode.ThemeIcon('sync~spin');
                item.description = r.phase || '';
                return item;
            });
        }
        catch {
            return [new HydraTreeItem('Unable to fetch', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
    }
    async getSisterItems() {
        const running = await this.client.isServerRunning();
        if (!running) {
            return [new HydraTreeItem('Server offline', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
        try {
            const result = await this.client.rpc('sisters', {});
            const sisters = result?.sisters || [];
            return sisters.map((s) => {
                const item = new HydraTreeItem(s.name, vscode.TreeItemCollapsibleState.None, 'sister');
                item.iconPath = new vscode.ThemeIcon(s.connected ? 'check' : 'close');
                item.description = s.connected ? 'connected' : 'offline';
                return item;
            });
        }
        catch {
            return [new HydraTreeItem('Unable to fetch', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
    }
    async getRecentItems() {
        return [new HydraTreeItem('No recent tasks', vscode.TreeItemCollapsibleState.None, 'empty')];
    }
}
exports.HydraTreeViewProvider = HydraTreeViewProvider;
class HydraTreeItem extends vscode.TreeItem {
    constructor(label, collapsibleState, contextValue) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.contextValue = contextValue;
    }
}
//# sourceMappingURL=treeViewProvider.js.map