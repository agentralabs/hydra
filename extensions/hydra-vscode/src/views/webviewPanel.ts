import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { HydraClient } from '../hydraClient';

export class HydraWebviewPanel {
    public static currentPanel: HydraWebviewPanel | undefined;
    private readonly panel: vscode.WebviewPanel;
    private readonly client: HydraClient;
    private readonly extensionUri: vscode.Uri;

    public static createOrShow(extensionUri: vscode.Uri, client: HydraClient): void {
        const column = vscode.window.activeTextEditor?.viewColumn;

        if (HydraWebviewPanel.currentPanel) {
            HydraWebviewPanel.currentPanel.panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            'hydraWorkspace',
            'Hydra Workspace',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                retainContextWhenHidden: true,
                localResourceRoots: [
                    vscode.Uri.joinPath(extensionUri, 'media'),
                    // Allow access to shared UI components
                    vscode.Uri.file(path.join(extensionUri.fsPath, '..', '..', 'shared', 'hydra-ui')),
                ],
            }
        );

        HydraWebviewPanel.currentPanel = new HydraWebviewPanel(panel, client, extensionUri);
    }

    private constructor(panel: vscode.WebviewPanel, client: HydraClient, extensionUri: vscode.Uri) {
        this.panel = panel;
        this.client = client;
        this.extensionUri = extensionUri;
        this.panel.webview.html = this.getHtml();

        this.panel.webview.onDidReceiveMessage(async (msg) => {
            switch (msg.command) {
                case 'query':
                    try {
                        const result = await this.client.rpc('hydra.run', { input: msg.text });
                        this.panel.webview.postMessage({ type: 'response', data: result });
                    } catch (e: any) {
                        this.panel.webview.postMessage({ type: 'error', message: e.message });
                    }
                    break;
                case 'approve':
                    await this.client.rpc('hydra.approve', { run_id: msg.id, decision: 'approved' });
                    break;
                case 'deny':
                    await this.client.rpc('hydra.approve', { run_id: msg.id, decision: 'denied' });
                    break;
                case 'applyDiff':
                    // Open diff in VS Code native diff editor
                    const uri = vscode.Uri.file(msg.filePath);
                    vscode.commands.executeCommand('vscode.diff', uri, uri, `Hydra: ${msg.filePath}`);
                    break;
                case 'openFile':
                    const fileUri = vscode.Uri.file(msg.filePath);
                    vscode.window.showTextDocument(fileUri, { preview: false });
                    break;
                case 'copyToClipboard':
                    vscode.env.clipboard.writeText(msg.text);
                    vscode.window.showInformationMessage('Copied to clipboard');
                    break;
            }
        });

        this.panel.onDidDispose(() => {
            HydraWebviewPanel.currentPanel = undefined;
        });

        // Subscribe to SSE for real-time updates
        this.startSSE();
    }

    private async startSSE(): Promise<void> {
        try {
            const baseUrl = this.client.getBaseUrl?.() || 'http://127.0.0.1:7777';
            // SSE subscription handled via periodic polling in webview
            // since VS Code webviews can't directly connect to SSE
            this.panel.webview.postMessage({ type: 'sseConnected', baseUrl });
        } catch {
            // Server not available
        }
    }

    /** Send a streaming chunk to the webview */
    public postStreamChunk(chunk: any): void {
        this.panel.webview.postMessage({ type: 'streamChunk', chunk });
    }

    /** Send a thinking update */
    public postThinking(verb: string, sister?: string): void {
        this.panel.webview.postMessage({ type: 'thinking', verb, sister });
    }

    /** Send morning briefing */
    public postBriefing(items: any[]): void {
        this.panel.webview.postMessage({ type: 'briefing', items });
    }

    /** Send approval request */
    public postApproval(approval: any): void {
        this.panel.webview.postMessage({ type: 'approval', card: approval });
    }

    private loadSharedCSS(): string {
        try {
            const cssPath = path.join(this.extensionUri.fsPath, '..', '..', 'shared', 'hydra-ui', 'css', 'hydra-patterns.css');
            return fs.readFileSync(cssPath, 'utf8');
        } catch {
            return '/* shared CSS not found */';
        }
    }

    private loadSharedJS(): string {
        try {
            const jsPath = path.join(this.extensionUri.fsPath, '..', '..', 'shared', 'hydra-ui', 'js', 'hydra-streaming.js');
            return fs.readFileSync(jsPath, 'utf8');
        } catch {
            return '/* shared JS not found */';
        }
    }

    private getHtml(): string {
        const sharedCSS = this.loadSharedCSS();
        const sharedJS = this.loadSharedJS();

        return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
/* Shared Hydra patterns CSS */
${sharedCSS}

/* VS Code-specific overrides */
body {
    font-family: var(--vscode-font-family);
    padding: 0; margin: 0;
    color: var(--vscode-foreground);
    background: var(--vscode-editor-background);
    height: 100vh;
    display: flex;
    flex-direction: column;
}
#chat-container {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
}
#thinking-area { padding: 0 12px; }
#approval-area { padding: 0 12px; }
#briefing-area { padding: 0 12px; }
</style>
</head>
<body>
    <div id="briefing-area"></div>
    <div id="chat-container"></div>
    <div id="thinking-area"></div>
    <div id="approval-area"></div>
    <div class="input-area">
        <input type="text" id="chat-input" placeholder="Ask Hydra... (! for bash, / for commands)" />
        <button class="send-btn" onclick="send()">Send</button>
    </div>

<script>
/* Shared Hydra streaming + rendering engine */
${sharedJS}

const vscode = acquireVsCodeApi();
const chatContainer = document.getElementById('chat-container');
const thinkingArea = document.getElementById('thinking-area');
const approvalArea = document.getElementById('approval-area');
const briefingArea = document.getElementById('briefing-area');

// Streaming engine instance
const streamer = new HydraStreamer(chatContainer);

// Thinking state
let thinkingVerb = 'Thinking';
let thinkingStart = null;
let thinkingTokens = 0;
let thinkingTipIdx = 0;
let thinkingInterval = null;
const tips = [
    'Use /btw to ask a quick side question without interrupting',
    'Use /beliefs to see what Hydra knows',
    'Use /roi to track value generated',
    'Dream State will test your beliefs overnight',
    'Use /undo to revert the last edit',
    '/compact preserves key decisions, frees context',
];

function send() {
    const input = document.getElementById('chat-input');
    if (!input.value.trim()) return;
    const text = input.value;
    input.value = '';

    // Render user message
    addMessage('user', text);

    // Send to extension
    vscode.postMessage({ command: 'query', text });

    // Show thinking
    startThinking('Thinking');
}

function addMessage(role, content) {
    const div = document.createElement('div');
    div.className = 'message message-' + role;
    if (role === 'user') {
        div.innerHTML = '<div class="user-label">You</div>' + HydraMarkdown.render(content);
    } else if (role === 'assistant') {
        div.innerHTML = HydraMarkdown.render(content);
    } else {
        div.innerHTML = '<div class="message-system">' + HydraMarkdown.render(content) + '</div>';
    }
    chatContainer.appendChild(div);
    chatContainer.scrollTop = chatContainer.scrollHeight;
}

function startThinking(verb, sister) {
    thinkingVerb = verb || 'Thinking';
    thinkingStart = Date.now();
    thinkingTokens = 0;

    if (thinkingInterval) clearInterval(thinkingInterval);
    thinkingInterval = setInterval(updateThinking, 1000);
    updateThinking();
}

function updateThinking() {
    if (!thinkingStart) { thinkingArea.innerHTML = ''; return; }
    const elapsed = Math.floor((Date.now() - thinkingStart) / 1000);
    const elapsedStr = elapsed >= 60
        ? Math.floor(elapsed / 60) + 'm ' + (elapsed % 60) + 's'
        : elapsed + 's';
    const tokenStr = thinkingTokens > 1000
        ? (thinkingTokens / 1000).toFixed(1) + 'k'
        : thinkingTokens;
    const tip = tips[thinkingTipIdx % tips.length];

    thinkingArea.innerHTML = '<div class="thinking-indicator">' +
        '<div class="thinking-spinner"></div>' +
        '<span>✱ ' + thinkingVerb + ' (' + elapsedStr +
        (thinkingTokens > 0 ? ' · ↓ ' + tokenStr + ' tokens' : '') + ')</span>' +
        '</div>' +
        '<div class="thinking-tip">└ Tip: ' + tip + '</div>';

    // Rotate tip every 15s
    if (elapsed > 0 && elapsed % 15 === 0) thinkingTipIdx++;
}

function stopThinking() {
    thinkingStart = null;
    if (thinkingInterval) clearInterval(thinkingInterval);
    thinkingArea.innerHTML = '';
}

function showApproval(card) {
    const level = card.risk || 'medium';
    approvalArea.innerHTML =
        '<div class="approval-modal risk-' + level + '">' +
        '<span class="risk-badge ' + level + '">' + level.toUpperCase() + '</span> ' +
        '<strong>' + card.title + '</strong>' +
        '<p>' + (card.description || '') + '</p>' +
        (card.diff ? HydraDiff.render(card.filePath || '', card.diff, { applyable: true }) : '') +
        '<div class="approval-actions">' +
        '<button class="btn-allow" onclick="approve(\\'' + card.id + '\\')">Allow (Y)</button>' +
        '<button class="btn-deny" onclick="deny(\\'' + card.id + '\\')">Deny (N)</button>' +
        '<button class="btn-allow-all" onclick="approveAll(\\'' + card.id + '\\')">Allow all this session</button>' +
        '</div>' +
        '<label class="approval-checkbox"><input type="checkbox"> Always allow for this file type</label>' +
        '</div>';
}

function approve(id) { vscode.postMessage({command:'approve',id}); approvalArea.innerHTML=''; }
function deny(id) { vscode.postMessage({command:'deny',id}); approvalArea.innerHTML=''; }
function approveAll(id) { approve(id); /* TODO: store session auto-approve */ }
function applyDiff(filePath) { vscode.postMessage({command:'applyDiff',filePath}); }
function openFile(filePath) { vscode.postMessage({command:'openFile',filePath}); }

// Handle messages from extension
window.addEventListener('message', event => {
    const msg = event.data;
    switch (msg.type) {
        case 'response':
            stopThinking();
            if (msg.data?.output) addMessage('assistant', msg.data.output);
            else if (msg.data?.result) addMessage('assistant', JSON.stringify(msg.data.result, null, 2));
            break;
        case 'error':
            stopThinking();
            addMessage('system', 'Error: ' + msg.message);
            break;
        case 'streamChunk':
            handleStreamChunk(msg.chunk);
            break;
        case 'thinking':
            startThinking(msg.verb, msg.sister);
            break;
        case 'approval':
            showApproval(msg.card);
            break;
        case 'briefing':
            briefingArea.innerHTML = HydraBriefing.render(msg.items);
            break;
    }
});

function handleStreamChunk(chunk) {
    switch (chunk.type) {
        case 'text':
            if (chunk.content) {
                if (!streamer.active) {
                    streamer.start('text');
                }
                streamer.append(chunk.content);
                thinkingTokens += Math.ceil((chunk.content.length) / 4);
            }
            break;
        case 'thinking':
        case 'tool_start':
            startThinking(chunk.sister ? pickSisterVerb(chunk.sister) : 'Thinking');
            break;
        case 'tool_end':
            // Show tool result
            if (chunk.sister && chunk.tool) {
                const dur = chunk.duration_ms ? (chunk.duration_ms / 1000).toFixed(1) + 's' : '';
                const dotClass = 'dot-success';
                const html = '<div class="tool-result"><div class="tool-header">' +
                    '<span class="dot ' + dotClass + '"></span>' +
                    '<span class="tool-sister">' + chunk.sister + '</span>' +
                    '<span class="tool-connector"> ▸ </span>' +
                    '<span class="tool-action">' + chunk.tool + '</span>' +
                    '<span class="tool-duration">' + dur + '</span>' +
                    '</div></div>';
                chatContainer.insertAdjacentHTML('beforeend', html);
            }
            break;
        case 'done':
            stopThinking();
            if (streamer.active) {
                streamer.stop();
                // Wrap final content as assistant message
            }
            break;
        case 'error':
            stopThinking();
            if (chunk.content) addMessage('system', 'Error: ' + chunk.content);
            break;
    }
}

function pickSisterVerb(sister) {
    const verbs = {
        Memory:'Remembering', Codebase:'Scanning', Data:'Crunching', Connect:'Reaching',
        Forge:'Forging', Workflow:'Orchestrating', Veritas:'Verifying', Aegis:'Shielding',
        Evolve:'Crystallizing', Vision:'Perceiving', Identity:'Authenticating',
        Time:'Scheduling', Contract:'Reviewing', Planning:'Strategizing',
        Cognition:'Modeling', Reality:'Probing', Comm:'Dispatching',
    };
    return verbs[sister] || 'Thinking';
}

// Keyboard shortcuts
document.getElementById('chat-input').addEventListener('keypress', e => {
    if (e.key === 'Enter') send();
});
document.addEventListener('keydown', e => {
    if (e.key === 'y' || e.key === 'Y') {
        const btn = document.querySelector('.btn-allow');
        if (btn) btn.click();
    }
    if (e.key === 'n' || e.key === 'N') {
        const btn = document.querySelector('.btn-deny');
        if (btn) btn.click();
    }
    if (e.key === 'Escape') {
        approvalArea.innerHTML = '';
        briefingArea.innerHTML = '';
    }
});
</script>
</body>
</html>`;
    }
}
