import * as vscode from 'vscode';
import { HydraClient } from '../hydraClient';

export class HydraTreeViewProvider implements vscode.TreeDataProvider<HydraTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<HydraTreeItem | undefined>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private client: HydraClient) {}

    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }

    getTreeItem(element: HydraTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: HydraTreeItem): Promise<HydraTreeItem[]> {
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

    private async getStatusItems(): Promise<HydraTreeItem[]> {
        const connected = await this.client.isServerRunning();
        const item = new HydraTreeItem(
            connected ? 'Connected' : 'Disconnected',
            vscode.TreeItemCollapsibleState.None,
            'status-item'
        );
        item.iconPath = new vscode.ThemeIcon(connected ? 'circle-filled' : 'circle-outline');
        return [item];
    }

    private async getRunItems(): Promise<HydraTreeItem[]> {
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
            return runs.map((r: any) => {
                const item = new HydraTreeItem(r.intent || 'Run', vscode.TreeItemCollapsibleState.None, 'run');
                item.iconPath = new vscode.ThemeIcon('sync~spin');
                item.description = r.phase || '';
                return item;
            });
        } catch {
            return [new HydraTreeItem('Unable to fetch', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
    }

    private async getSisterItems(): Promise<HydraTreeItem[]> {
        const running = await this.client.isServerRunning();
        if (!running) {
            return [new HydraTreeItem('Server offline', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
        try {
            const result = await this.client.rpc('sisters', {});
            const sisters = result?.sisters || [];
            return sisters.map((s: any) => {
                const item = new HydraTreeItem(s.name, vscode.TreeItemCollapsibleState.None, 'sister');
                item.iconPath = new vscode.ThemeIcon(s.connected ? 'check' : 'close');
                item.description = s.connected ? 'connected' : 'offline';
                return item;
            });
        } catch {
            return [new HydraTreeItem('Unable to fetch', vscode.TreeItemCollapsibleState.None, 'empty')];
        }
    }

    private async getRecentItems(): Promise<HydraTreeItem[]> {
        return [new HydraTreeItem('No recent tasks', vscode.TreeItemCollapsibleState.None, 'empty')];
    }
}

class HydraTreeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly contextValue: string,
    ) {
        super(label, collapsibleState);
    }
}
