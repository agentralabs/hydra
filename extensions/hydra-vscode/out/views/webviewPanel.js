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
exports.HydraWebviewPanel = void 0;
const vscode = __importStar(require("vscode"));
class HydraWebviewPanel {
    static createOrShow(extensionUri, client) {
        const column = vscode.window.activeTextEditor?.viewColumn;
        if (HydraWebviewPanel.currentPanel) {
            HydraWebviewPanel.currentPanel.panel.reveal(column);
            return;
        }
        const panel = vscode.window.createWebviewPanel('hydraWorkspace', 'Hydra Workspace', column || vscode.ViewColumn.One, { enableScripts: true, retainContextWhenHidden: true });
        HydraWebviewPanel.currentPanel = new HydraWebviewPanel(panel, client);
    }
    constructor(panel, client) {
        this.panel = panel;
        this.client = client;
        this.panel.webview.html = this.getHtml();
        this.panel.webview.onDidReceiveMessage(async (msg) => {
            switch (msg.command) {
                case 'query':
                    try {
                        const result = await this.client.rpc('run', { intent: msg.text });
                        this.panel.webview.postMessage({ type: 'response', data: result });
                    }
                    catch (e) {
                        this.panel.webview.postMessage({ type: 'error', message: e.message });
                    }
                    break;
                case 'approve':
                    await this.client.rpc('approve', { run_id: msg.id });
                    break;
                case 'deny':
                    await this.client.rpc('deny', { run_id: msg.id });
                    break;
            }
        });
        this.panel.onDidDispose(() => {
            HydraWebviewPanel.currentPanel = undefined;
        });
    }
    getHtml() {
        return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<style>
    :root {
        --trust-blue: #4A9EFF;
        --success: #4ADE80;
        --attention: #FFAA4A;
        --error: #FF6B6B;
    }
    body {
        font-family: var(--vscode-font-family);
        padding: 0;
        margin: 0;
        color: var(--vscode-foreground);
        background: var(--vscode-editor-background);
        height: 100vh;
        display: flex;
        flex-direction: column;
    }
    .workspace {
        display: grid;
        grid-template-columns: 1fr 1.5fr 1fr;
        gap: 12px;
        padding: 12px;
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }
    .panel {
        background: var(--vscode-sideBar-background);
        border: 1px solid var(--vscode-panel-border);
        border-radius: 8px;
        padding: 12px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
    }
    .panel h3 {
        font-size: 11px;
        text-transform: uppercase;
        letter-spacing: 1.5px;
        color: var(--trust-blue);
        margin: 0 0 8px 0;
        font-weight: 700;
    }
    .panel-empty {
        font-size: 12px;
        color: var(--vscode-descriptionForeground);
        font-style: italic;
        text-align: center;
        padding: 20px 0;
    }
    .plan-step {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 12px;
        padding: 3px 0;
    }
    .plan-step .icon { width: 16px; text-align: center; }
    .plan-step.completed { color: var(--success); }
    .plan-step.running { color: var(--trust-blue); font-weight: 600; }
    .plan-step.pending { color: var(--vscode-descriptionForeground); }
    .timeline-event {
        display: flex;
        gap: 8px;
        font-size: 11px;
        padding: 4px 0;
        border-left: 2px solid var(--vscode-panel-border);
        padding-left: 8px;
        margin-left: 4px;
    }
    .timeline-event .time {
        font-family: monospace;
        color: var(--vscode-descriptionForeground);
        min-width: 50px;
    }
    .evidence-item {
        background: var(--vscode-editor-background);
        border: 1px solid var(--vscode-panel-border);
        border-radius: 6px;
        padding: 8px;
        margin-bottom: 8px;
        font-size: 11px;
    }
    .evidence-item .title {
        font-weight: 600;
        margin-bottom: 4px;
    }
    .evidence-item .content {
        font-family: monospace;
        font-size: 10px;
        color: var(--vscode-descriptionForeground);
        white-space: pre-wrap;
        max-height: 80px;
        overflow-y: auto;
    }
    .chat-area {
        border-top: 1px solid var(--vscode-panel-border);
        padding: 12px;
        display: flex;
        gap: 8px;
    }
    .chat-area input {
        flex: 1;
        padding: 8px 14px;
        border-radius: 20px;
        border: 1px solid var(--vscode-input-border);
        background: var(--vscode-input-background);
        color: var(--vscode-input-foreground);
        font-size: 13px;
        outline: none;
    }
    .chat-area button {
        padding: 8px 16px;
        border-radius: 20px;
        border: none;
        background: var(--trust-blue);
        color: white;
        cursor: pointer;
        font-size: 13px;
    }
    .approval-card {
        background: rgba(255, 170, 74, 0.1);
        border: 1px solid var(--attention);
        border-radius: 8px;
        padding: 12px;
        margin: 8px 12px;
    }
    .approval-card h4 { margin: 0 0 6px; font-size: 13px; }
    .approval-card p { margin: 0 0 8px; font-size: 12px; color: var(--vscode-descriptionForeground); }
    .approval-actions { display: flex; gap: 8px; }
    .approval-actions button { padding: 6px 14px; border-radius: 14px; border: none; cursor: pointer; font-size: 12px; }
    .btn-approve { background: var(--success); color: white; }
    .btn-deny { background: var(--error); color: white; }
</style>
</head>
<body>
    <div class="workspace">
        <div class="panel" id="plan-panel">
            <h3>Plan</h3>
            <div id="plan-content"><p class="panel-empty">No active plan</p></div>
        </div>
        <div class="panel" id="timeline-panel">
            <h3>Timeline</h3>
            <div id="timeline-content"><p class="panel-empty">No events yet</p></div>
        </div>
        <div class="panel" id="evidence-panel">
            <h3>Evidence</h3>
            <div id="evidence-content"><p class="panel-empty">No evidence collected</p></div>
        </div>
    </div>
    <div id="approval-container"></div>
    <div class="chat-area">
        <input type="text" id="chat-input" placeholder="Ask Hydra..." />
        <button onclick="send()">Send</button>
    </div>
    <script>
        const vscode = acquireVsCodeApi();
        function send() {
            const input = document.getElementById('chat-input');
            if (!input.value.trim()) return;
            vscode.postMessage({ command: 'query', text: input.value });
            addTimeline('Sent: ' + input.value);
            input.value = '';
        }
        document.getElementById('chat-input').addEventListener('keypress', e => {
            if (e.key === 'Enter') send();
        });
        function addTimeline(text) {
            const el = document.getElementById('timeline-content');
            const empty = el.querySelector('.panel-empty');
            if (empty) empty.remove();
            const now = new Date().toLocaleTimeString('en-US', {hour12:false, hour:'2-digit', minute:'2-digit', second:'2-digit'});
            el.innerHTML += '<div class="timeline-event"><span class="time">' + now + '</span><span>' + text + '</span></div>';
            el.scrollTop = el.scrollHeight;
        }
        function showApproval(card) {
            document.getElementById('approval-container').innerHTML =
                '<div class="approval-card"><h4>' + card.title + '</h4><p>' + card.description +
                '</p><div class="approval-actions"><button class="btn-approve" onclick="approve(\\'' +
                card.id + '\\')">Approve (Y)</button><button class="btn-deny" onclick="deny(\\'' +
                card.id + '\\')">Deny (N)</button></div></div>';
        }
        function approve(id) { vscode.postMessage({command:'approve',id}); document.getElementById('approval-container').innerHTML=''; }
        function deny(id) { vscode.postMessage({command:'deny',id}); document.getElementById('approval-container').innerHTML=''; }
        window.addEventListener('message', event => {
            const msg = event.data;
            if (msg.type === 'response') {
                addTimeline('Response received');
                if (msg.data?.plan) updatePlan(msg.data.plan);
                if (msg.data?.evidence) updateEvidence(msg.data.evidence);
            }
            if (msg.type === 'approval') showApproval(msg.card);
            if (msg.type === 'error') addTimeline('Error: ' + msg.message);
        });
        function updatePlan(plan) {
            const el = document.getElementById('plan-content');
            let html = plan.steps ? plan.steps.map((s,i) =>
                '<div class="plan-step ' + (s.done ? 'completed' : s.current ? 'running' : 'pending') + '"><span class="icon">' +
                (s.done ? '\\u2713' : s.current ? '\\u25d0' : '\\u25cb') + '</span><span>' + s.label + '</span></div>'
            ).join('') : '';
            el.innerHTML = html || '<p class="panel-empty">No active plan</p>';
        }
        function updateEvidence(items) {
            const el = document.getElementById('evidence-content');
            el.innerHTML = items.map(e =>
                '<div class="evidence-item"><div class="title">' + e.title + '</div><div class="content">' + (e.content || '') + '</div></div>'
            ).join('') || '<p class="panel-empty">No evidence</p>';
        }
        document.addEventListener('keydown', e => {
            if (e.key === 'y' || e.key === 'Y') {
                const approveBtn = document.querySelector('.btn-approve');
                if (approveBtn) approveBtn.click();
            }
            if (e.key === 'n' || e.key === 'N') {
                const denyBtn = document.querySelector('.btn-deny');
                if (denyBtn) denyBtn.click();
            }
        });
    </script>
</body>
</html>`;
    }
}
exports.HydraWebviewPanel = HydraWebviewPanel;
//# sourceMappingURL=webviewPanel.js.map