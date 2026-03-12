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
exports.HydraSidebarProvider = void 0;
const vscode = __importStar(require("vscode"));
class HydraSidebarProvider {
    constructor(extensionUri, client) {
        this.extensionUri = extensionUri;
        this.client = client;
        this.status = null;
        this.runs = [];
        this.approvals = [];
        this.sisters = [];
    }
    resolveWebviewView(webviewView, _context, _token) {
        this.view = webviewView;
        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.extensionUri],
        };
        webviewView.webview.html = this.getHtml();
        webviewView.webview.onDidReceiveMessage(async (msg) => {
            switch (msg.type) {
                case 'run': {
                    const intent = msg.intent;
                    if (intent) {
                        try {
                            await this.client.createRun(intent);
                            vscode.window.showInformationMessage(`Hydra: Working on "${intent}"...`);
                        }
                        catch (err) {
                            vscode.window.showErrorMessage(`Hydra: Failed to start run - ${err}`);
                        }
                    }
                    break;
                }
                case 'stop': {
                    try {
                        await this.client.killAll();
                        vscode.window.showWarningMessage('Hydra: All runs stopped');
                    }
                    catch (err) {
                        vscode.window.showErrorMessage(`Hydra: Failed to stop - ${err}`);
                    }
                    break;
                }
                case 'approve': {
                    try {
                        await this.client.approve(msg.runId, msg.actionId);
                        vscode.window.showInformationMessage('Hydra: Approved');
                    }
                    catch (err) {
                        vscode.window.showErrorMessage(`Hydra: Approval failed - ${err}`);
                    }
                    break;
                }
                case 'deny': {
                    try {
                        await this.client.deny(msg.runId, msg.actionId);
                        vscode.window.showInformationMessage('Hydra: Denied');
                    }
                    catch (err) {
                        vscode.window.showErrorMessage(`Hydra: Denial failed - ${err}`);
                    }
                    break;
                }
                case 'refresh': {
                    await this.refreshData();
                    break;
                }
            }
        });
    }
    async refreshData() {
        try {
            this.status = await this.client.getStatus();
            this.runs = await this.client.getActiveRuns();
            this.approvals = await this.client.getPendingApprovals();
            this.sisters = await this.client.getSisters();
        }
        catch {
            this.status = null;
            this.runs = [];
            this.approvals = [];
            this.sisters = [];
        }
        if (this.view) {
            this.view.webview.postMessage({
                type: 'update',
                status: this.status,
                runs: this.runs,
                approvals: this.approvals,
                sisters: this.sisters,
            });
        }
    }
    refresh() {
        if (this.view) {
            this.view.webview.html = this.getHtml();
        }
    }
    getHtml() {
        return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, sans-serif);
    font-size: var(--vscode-font-size, 13px);
    color: var(--vscode-foreground, #cccccc);
    background: var(--vscode-sideBar-background, #1e1e1e);
    padding: 12px;
  }
  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--vscode-panel-border, #333);
  }
  .header svg { width: 20px; height: 20px; flex-shrink: 0; }
  .header h2 { font-size: 14px; font-weight: 600; }
  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 500;
    margin-left: auto;
  }
  .status-badge.connected { background: #1a3a1a; color: #4EC94E; }
  .status-badge.offline { background: #3a2a1a; color: #888; }
  .section { margin-bottom: 16px; }
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--vscode-descriptionForeground, #999);
    margin-bottom: 8px;
  }
  .stat-row {
    display: flex;
    justify-content: space-between;
    padding: 4px 0;
    font-size: 12px;
  }
  .stat-value { font-weight: 600; color: var(--vscode-foreground, #ccc); }
  .input-row {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
  }
  .input-row input {
    flex: 1;
    background: var(--vscode-input-background, #3c3c3c);
    color: var(--vscode-input-foreground, #ccc);
    border: 1px solid var(--vscode-input-border, #555);
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 12px;
    outline: none;
  }
  .input-row input:focus {
    border-color: var(--vscode-focusBorder, #4A9EFF);
  }
  .input-row input::placeholder {
    color: var(--vscode-input-placeholderForeground, #888);
  }
  button {
    background: var(--vscode-button-background, #0e639c);
    color: var(--vscode-button-foreground, #fff);
    border: none;
    border-radius: 4px;
    padding: 6px 12px;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }
  button:hover {
    background: var(--vscode-button-hoverBackground, #1177bb);
  }
  button.secondary {
    background: var(--vscode-button-secondaryBackground, #3a3d41);
    color: var(--vscode-button-secondaryForeground, #ccc);
  }
  button.secondary:hover {
    background: var(--vscode-button-secondaryHoverBackground, #45494e);
  }
  button.danger {
    background: #5a1d1d;
    color: #f88;
  }
  button.danger:hover {
    background: #7a2d2d;
  }
  .btn-row {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
  }
  .btn-row button { flex: 1; }
  .run-card, .approval-card {
    background: var(--vscode-editor-background, #1e1e1e);
    border: 1px solid var(--vscode-panel-border, #333);
    border-radius: 6px;
    padding: 10px;
    margin-bottom: 8px;
  }
  .run-card .run-intent {
    font-weight: 600;
    font-size: 12px;
    margin-bottom: 4px;
  }
  .run-card .run-meta {
    font-size: 11px;
    color: var(--vscode-descriptionForeground, #999);
  }
  .approval-card { border-color: #FF8C00; }
  .approval-card .risk {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    margin-bottom: 4px;
  }
  .approval-card .risk.low { color: #4EC94E; }
  .approval-card .risk.medium { color: #FFCC00; }
  .approval-card .risk.high { color: #FF8C00; }
  .approval-card .risk.critical { color: #F44747; }
  .approval-card .summary {
    font-size: 12px;
    margin-bottom: 8px;
  }
  .approval-actions {
    display: flex;
    gap: 6px;
  }
  .approval-actions button { flex: 1; font-size: 11px; padding: 4px 8px; }
  .sister-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-size: 12px;
  }
  .sister-dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .sister-dot.on { background: #4EC94E; }
  .sister-dot.off { background: #F44747; }
  .empty {
    font-size: 12px;
    color: var(--vscode-descriptionForeground, #888);
    font-style: italic;
    padding: 8px 0;
  }
</style>
</head>
<body>
  <div class="header">
    <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
      <circle cx="12" cy="12" r="10" fill="none" stroke="#4A9EFF" stroke-width="2"/>
      <circle cx="12" cy="12" r="5" fill="#4A9EFF"/>
    </svg>
    <h2>Hydra</h2>
    <span id="connectionBadge" class="status-badge offline">Offline</span>
  </div>

  <div class="section">
    <div class="input-row">
      <input id="intentInput" type="text" placeholder="What should Hydra do?" />
      <button id="runBtn">Run</button>
    </div>
    <div class="btn-row">
      <button id="stopBtn" class="danger">Stop All</button>
      <button id="refreshBtn" class="secondary">Refresh</button>
    </div>
  </div>

  <div class="section">
    <div class="section-title">Status</div>
    <div class="stat-row"><span>Active Runs</span><span id="activeRuns" class="stat-value">0</span></div>
    <div class="stat-row"><span>Pending Approvals</span><span id="pendingApprovals" class="stat-value">0</span></div>
    <div class="stat-row"><span>Phase</span><span id="phase" class="stat-value">--</span></div>
    <div class="stat-row"><span>Tokens Today</span><span id="tokensToday" class="stat-value">0</span></div>
  </div>

  <div class="section">
    <div class="section-title">Approvals</div>
    <div id="approvalsList"><p class="empty">No pending approvals</p></div>
  </div>

  <div class="section">
    <div class="section-title">Active Runs</div>
    <div id="runsList"><p class="empty">No active runs</p></div>
  </div>

  <div class="section">
    <div class="section-title">Sisters</div>
    <div id="sistersList"><p class="empty">No sister data</p></div>
  </div>

  <script>
    const vscode = acquireVsCodeApi();

    document.getElementById('runBtn').addEventListener('click', () => {
      const input = document.getElementById('intentInput');
      const intent = input.value.trim();
      if (intent) {
        vscode.postMessage({ type: 'run', intent });
        input.value = '';
      }
    });

    document.getElementById('intentInput').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        document.getElementById('runBtn').click();
      }
    });

    document.getElementById('stopBtn').addEventListener('click', () => {
      vscode.postMessage({ type: 'stop' });
    });

    document.getElementById('refreshBtn').addEventListener('click', () => {
      vscode.postMessage({ type: 'refresh' });
    });

    window.addEventListener('message', (event) => {
      const msg = event.data;
      if (msg.type === 'update') {
        updateUI(msg.status, msg.runs, msg.approvals, msg.sisters);
      }
    });

    function updateUI(status, runs, approvals, sisters) {
      const badge = document.getElementById('connectionBadge');
      if (status) {
        badge.textContent = 'Connected';
        badge.className = 'status-badge connected';
        document.getElementById('activeRuns').textContent = status.activeRuns;
        document.getElementById('pendingApprovals').textContent = status.pendingApprovals;
        document.getElementById('phase').textContent = status.phase || '--';
        document.getElementById('tokensToday').textContent = status.tokensToday.toLocaleString();
      } else {
        badge.textContent = 'Offline';
        badge.className = 'status-badge offline';
        document.getElementById('activeRuns').textContent = '0';
        document.getElementById('pendingApprovals').textContent = '0';
        document.getElementById('phase').textContent = '--';
        document.getElementById('tokensToday').textContent = '0';
      }

      // Approvals
      const approvalsList = document.getElementById('approvalsList');
      if (approvals && approvals.length > 0) {
        approvalsList.innerHTML = approvals.map(a =>
          '<div class="approval-card">' +
            '<div class="risk ' + a.riskLevel + '">' + a.riskLevel + ' risk</div>' +
            '<div class="summary">' + escapeHtml(a.summary) + '</div>' +
            '<div class="approval-actions">' +
              '<button onclick="handleApprove(\'' + a.runId + '\', \'' + a.id + '\')">Approve</button>' +
              '<button class="danger" onclick="handleDeny(\'' + a.runId + '\', \'' + a.id + '\')">Deny</button>' +
            '</div>' +
          '</div>'
        ).join('');
      } else {
        approvalsList.innerHTML = '<p class="empty">No pending approvals</p>';
      }

      // Runs
      const runsList = document.getElementById('runsList');
      if (runs && runs.length > 0) {
        runsList.innerHTML = runs.map(r =>
          '<div class="run-card">' +
            '<div class="run-intent">' + escapeHtml(r.intent) + '</div>' +
            '<div class="run-meta">' + r.status + ' &middot; ' + r.tokensUsed.toLocaleString() + ' tokens</div>' +
          '</div>'
        ).join('');
      } else {
        runsList.innerHTML = '<p class="empty">No active runs</p>';
      }

      // Sisters
      const sistersList = document.getElementById('sistersList');
      if (sisters && sisters.length > 0) {
        sistersList.innerHTML = sisters.map(s =>
          '<div class="sister-row">' +
            '<span class="sister-dot ' + (s.connected ? 'on' : 'off') + '"></span>' +
            '<span>' + escapeHtml(s.name) + '</span>' +
          '</div>'
        ).join('');
      } else {
        sistersList.innerHTML = '<p class="empty">No sister data</p>';
      }
    }

    function handleApprove(runId, actionId) {
      vscode.postMessage({ type: 'approve', runId, actionId });
    }

    function handleDeny(runId, actionId) {
      vscode.postMessage({ type: 'deny', runId, actionId });
    }

    function escapeHtml(str) {
      const div = document.createElement('div');
      div.textContent = str;
      return div.innerHTML;
    }

    // Request initial data
    vscode.postMessage({ type: 'refresh' });
  </script>
</body>
</html>`;
    }
}
exports.HydraSidebarProvider = HydraSidebarProvider;
HydraSidebarProvider.viewType = 'hydra.sidebar';
//# sourceMappingURL=sidebarProvider.js.map