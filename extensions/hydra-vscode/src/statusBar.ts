import * as vscode from 'vscode';
import { IconState } from './types';

/**
 * Hydra Status Bar — shows sister count, profile, cost, thinking verbs.
 * Spec: Pattern 19 (thinking verbs + spinner) + Hydra Pattern 5 (compiled patterns)
 * Format: "◉ Hydra | 17/17 | dev (66) | $0.04"
 * During thinking: "✱ Forging... (12s · ↓ 1.4k tokens · $0.008)"
 */
export class HydraStatusBar implements vscode.Disposable {
  private item: vscode.StatusBarItem;
  private compiledItem: vscode.StatusBarItem;
  private state: IconState = 'idle';

  // Live stats
  private sistersConnected = 0;
  private sistersTotal = 17;
  private profileName = '';
  private beliefsLoaded = 0;
  private sessionCost = 0;
  private compiledPatterns = 0;
  private compiledSavings = 0;

  // Thinking state
  private thinkingVerb = '';
  private thinkingStart: number | null = null;
  private thinkingTokens = 0;
  private thinkingInterval: NodeJS.Timeout | null = null;

  constructor() {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this.item.command = 'hydra.toggleSidebar';
    this.item.show();

    // Compiled patterns indicator (separate item, right-aligned)
    this.compiledItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 50);
    this.compiledItem.command = 'hydra.status';

    this.update('idle');
  }

  /** Update the full status display */
  update(state: IconState, detail?: string): void {
    this.state = state;

    if (state === 'working' && detail) {
      // Thinking mode — show verb + live stats
      this.startThinking(detail);
      return;
    }

    if (this.thinkingInterval) {
      this.stopThinking();
    }

    const icon = this.getIcon(state);
    const color = this.getColor(state);

    // Format: "◉ Hydra | 17/17 | dev (66) | $0.04"
    let parts = ['Hydra'];
    if (this.sistersConnected > 0) {
      parts.push(`${this.sistersConnected}/${this.sistersTotal}`);
    }
    if (this.profileName) {
      parts.push(`${this.profileName} (${this.beliefsLoaded})`);
    }
    if (this.sessionCost > 0) {
      parts.push(`$${this.sessionCost.toFixed(2)}`);
    }

    this.item.text = `${icon} ${parts.join(' | ')}`;
    this.item.tooltip = this.getTooltip(state);
    this.item.color = color;

    // Update compiled patterns indicator
    if (this.compiledPatterns > 0) {
      this.compiledItem.text = `⚡ ${this.compiledPatterns} compiled | saves $${this.compiledSavings.toFixed(2)}/day`;
      this.compiledItem.tooltip = 'Compiled patterns execute at 0 tokens. Click to see all.';
      this.compiledItem.show();
    } else {
      this.compiledItem.hide();
    }
  }

  /** Update sister/profile/cost stats */
  updateStats(stats: {
    sistersConnected?: number;
    sistersTotal?: number;
    profileName?: string;
    beliefsLoaded?: number;
    sessionCost?: number;
    compiledPatterns?: number;
    compiledSavings?: number;
  }): void {
    if (stats.sistersConnected !== undefined) this.sistersConnected = stats.sistersConnected;
    if (stats.sistersTotal !== undefined) this.sistersTotal = stats.sistersTotal;
    if (stats.profileName !== undefined) this.profileName = stats.profileName;
    if (stats.beliefsLoaded !== undefined) this.beliefsLoaded = stats.beliefsLoaded;
    if (stats.sessionCost !== undefined) this.sessionCost = stats.sessionCost;
    if (stats.compiledPatterns !== undefined) this.compiledPatterns = stats.compiledPatterns;
    if (stats.compiledSavings !== undefined) this.compiledSavings = stats.compiledSavings;

    // Refresh display if not thinking
    if (!this.thinkingInterval) {
      this.update(this.state);
    }
  }

  /** Start thinking display with context-aware verb */
  private startThinking(verb: string): void {
    this.thinkingVerb = verb;
    this.thinkingStart = Date.now();
    this.thinkingTokens = 0;

    if (this.thinkingInterval) clearInterval(this.thinkingInterval);
    this.thinkingInterval = setInterval(() => this.renderThinking(), 1000);
    this.renderThinking();
  }

  /** Stop thinking display */
  private stopThinking(): void {
    if (this.thinkingInterval) {
      clearInterval(this.thinkingInterval);
      this.thinkingInterval = null;
    }
    this.thinkingStart = null;
  }

  /** Update thinking tokens (called on each stream chunk) */
  addThinkingTokens(count: number): void {
    this.thinkingTokens += count;
  }

  private renderThinking(): void {
    if (!this.thinkingStart) return;

    const elapsed = Math.floor((Date.now() - this.thinkingStart) / 1000);
    const elapsedStr = elapsed >= 60
      ? `${Math.floor(elapsed / 60)}m ${elapsed % 60}s`
      : `${elapsed}s`;

    let tokenStr = '';
    if (this.thinkingTokens > 0) {
      tokenStr = this.thinkingTokens > 1000
        ? ` · ↓ ${(this.thinkingTokens / 1000).toFixed(1)}k tokens`
        : ` · ↓ ${this.thinkingTokens} tokens`;
    }

    const cost = this.thinkingTokens * 0.000003; // rough estimate
    const costStr = cost > 0.001 ? ` · $${cost.toFixed(3)}` : '';

    this.item.text = `$(sync~spin) ✱ ${this.thinkingVerb}... (${elapsedStr}${tokenStr}${costStr})`;
    this.item.color = '#F0A03C'; // orange for thinking
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

  private getTooltip(state: IconState): string {
    const base = `Hydra | ${this.sistersConnected}/${this.sistersTotal} sisters`;
    switch (state) {
      case 'idle':             return `${base} | Ready`;
      case 'listening':        return `${base} | Listening for input`;
      case 'working':          return `${base} | Executing`;
      case 'needsAttention':   return `${base} | Needs attention`;
      case 'approvalNeeded':   return `${base} | Pending approval`;
      case 'success':          return `${base} | Completed`;
      case 'error':            return `${base} | Error occurred`;
      case 'offline':          return 'Hydra server is not reachable';
    }
  }

  private getColor(state: IconState): string | undefined {
    switch (state) {
      case 'idle':             return '#6495ED'; // Hydra blue
      case 'listening':        return '#00D2D2'; // Hydra cyan
      case 'working':          return '#6495ED';
      case 'needsAttention':   return '#F0C850'; // Hydra yellow
      case 'approvalNeeded':   return '#F0A03C'; // Hydra orange
      case 'success':          return '#50C878'; // Hydra green
      case 'error':            return '#DC5050'; // Hydra red
      case 'offline':          return '#808080'; // Hydra dim
    }
  }

  dispose(): void {
    this.item.dispose();
    this.compiledItem.dispose();
    if (this.thinkingInterval) clearInterval(this.thinkingInterval);
  }
}
