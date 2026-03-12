import * as vscode from 'vscode';
import { IconState } from './types';

export class HydraStatusBar implements vscode.Disposable {
  private item: vscode.StatusBarItem;
  private state: IconState = 'idle';

  constructor() {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this.item.command = 'hydra.toggleSidebar';
    this.update('idle');
    this.item.show();
  }

  update(state: IconState, detail?: string): void {
    this.state = state;
    const icon = this.getIcon(state);
    const text = this.getText(state, detail);
    this.item.text = `${icon} ${text}`;
    this.item.tooltip = this.getTooltip(state);
    this.item.color = this.getColor(state);
  }

  getState(): IconState {
    return this.state;
  }

  private getIcon(state: IconState): string {
    switch (state) {
      case 'idle':             return '$(circle-filled)';
      case 'listening':        return '$(pulse)';
      case 'working':          return '$(sync~spin)';
      case 'needsAttention':   return '$(bell)';
      case 'approvalNeeded':   return '$(shield)';
      case 'success':          return '$(check)';
      case 'error':            return '$(error)';
      case 'offline':          return '$(circle-slash)';
    }
  }

  private getText(state: IconState, detail?: string): string {
    const base = 'Hydra';
    if (detail) {
      return `${base}: ${detail}`;
    }
    switch (state) {
      case 'idle':             return base;
      case 'listening':        return `${base}: Listening`;
      case 'working':          return `${base}: Working`;
      case 'needsAttention':   return `${base}: Attention`;
      case 'approvalNeeded':   return `${base}: Approval`;
      case 'success':          return `${base}: Done`;
      case 'error':            return `${base}: Error`;
      case 'offline':          return `${base}: Offline`;
    }
  }

  private getTooltip(state: IconState): string {
    switch (state) {
      case 'idle':             return 'Hydra is connected and idle';
      case 'listening':        return 'Hydra is listening for input';
      case 'working':          return 'Hydra is executing a run';
      case 'needsAttention':   return 'Hydra needs your attention';
      case 'approvalNeeded':   return 'Hydra has a pending approval';
      case 'success':          return 'Hydra completed successfully';
      case 'error':            return 'Hydra encountered an error';
      case 'offline':          return 'Hydra server is not reachable';
    }
  }

  private getColor(state: IconState): string | undefined {
    switch (state) {
      case 'idle':             return undefined;
      case 'listening':        return '#4A9EFF';
      case 'working':          return '#4A9EFF';
      case 'needsAttention':   return '#FFCC00';
      case 'approvalNeeded':   return '#FF8C00';
      case 'success':          return '#4EC94E';
      case 'error':            return '#F44747';
      case 'offline':          return '#888888';
    }
  }

  dispose(): void {
    this.item.dispose();
  }
}
