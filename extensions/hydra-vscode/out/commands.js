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
exports.registerCommands = registerCommands;
exports.handleToggleVoice = handleToggleVoice;
const vscode = __importStar(require("vscode"));
function registerCommands(context, client, statusBar) {
    const outputChannel = vscode.window.createOutputChannel('Hydra');
    context.subscriptions.push(vscode.commands.registerCommand('hydra.run', async () => {
        const intent = await vscode.window.showInputBox({
            prompt: 'What would you like Hydra to do?',
            placeHolder: 'e.g., "refactor the auth module"',
        });
        if (!intent) {
            return;
        }
        try {
            statusBar.update('working', intent);
            const run = await client.createRun(intent);
            vscode.window.showInformationMessage(`Hydra: Working on "${intent}" (run ${run.id})`);
        }
        catch (err) {
            statusBar.update('error');
            vscode.window.showErrorMessage(`Hydra: Failed to start run - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.status', async () => {
        try {
            const status = await client.getStatus();
            outputChannel.clear();
            outputChannel.appendLine('=== Hydra Status ===');
            outputChannel.appendLine(`Connected:         ${status.connected}`);
            outputChannel.appendLine(`Active Runs:       ${status.activeRuns}`);
            outputChannel.appendLine(`Pending Approvals: ${status.pendingApprovals}`);
            outputChannel.appendLine(`Phase:             ${status.phase ?? 'none'}`);
            outputChannel.appendLine(`Tokens Today:      ${status.tokensToday.toLocaleString()}`);
            outputChannel.show(true);
        }
        catch (err) {
            vscode.window.showErrorMessage(`Hydra: Cannot reach server - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.stop', async () => {
        const confirm = await vscode.window.showWarningMessage('Stop all Hydra runs?', { modal: true }, 'Stop All');
        if (confirm !== 'Stop All') {
            return;
        }
        try {
            await client.killAll();
            statusBar.update('idle');
            vscode.window.showWarningMessage('Hydra: All runs stopped');
        }
        catch (err) {
            vscode.window.showErrorMessage(`Hydra: Failed to stop - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.approve', async () => {
        try {
            const approvals = await client.getPendingApprovals();
            if (approvals.length === 0) {
                vscode.window.showInformationMessage('Hydra: No pending approvals');
                return;
            }
            const items = approvals.map(a => ({
                label: `[${a.riskLevel.toUpperCase()}] ${a.action}`,
                description: a.summary,
                approval: a,
            }));
            const picked = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select an action to approve',
            });
            if (picked) {
                await client.approve(picked.approval.runId, picked.approval.id);
                vscode.window.showInformationMessage('Hydra: Approved');
            }
        }
        catch (err) {
            vscode.window.showErrorMessage(`Hydra: ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.deny', async () => {
        try {
            const approvals = await client.getPendingApprovals();
            if (approvals.length === 0) {
                vscode.window.showInformationMessage('Hydra: No pending approvals');
                return;
            }
            const items = approvals.map(a => ({
                label: `[${a.riskLevel.toUpperCase()}] ${a.action}`,
                description: a.summary,
                approval: a,
            }));
            const picked = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select an action to deny',
            });
            if (picked) {
                await client.deny(picked.approval.runId, picked.approval.id);
                vscode.window.showInformationMessage('Hydra: Denied');
            }
        }
        catch (err) {
            vscode.window.showErrorMessage(`Hydra: ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.sisters', async () => {
        try {
            const sisters = await client.getSisters();
            outputChannel.clear();
            outputChannel.appendLine('=== Sister Status ===');
            for (const s of sisters) {
                const icon = s.connected ? '[OK]' : '[--]';
                const err = s.error ? ` (${s.error})` : '';
                outputChannel.appendLine(`  ${icon} ${s.name}${err}`);
            }
            outputChannel.show(true);
        }
        catch (err) {
            vscode.window.showErrorMessage(`Hydra: Cannot reach server - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.toggleSidebar', () => {
        vscode.commands.executeCommand('workbench.view.extension.hydra');
    }), vscode.commands.registerCommand('hydra.explain', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('Hydra: No active editor');
            return;
        }
        const selection = editor.selection;
        const code = selection.isEmpty
            ? editor.document.lineAt(selection.active.line).text
            : editor.document.getText(selection);
        if (!code.trim()) {
            vscode.window.showWarningMessage('Hydra: No code selected');
            return;
        }
        try {
            statusBar.update('working', 'Explaining...');
            const explanation = await client.explainCode(code, editor.document.languageId);
            outputChannel.clear();
            outputChannel.appendLine('=== Hydra: Explanation ===');
            outputChannel.appendLine('');
            outputChannel.appendLine(explanation);
            outputChannel.show(true);
            statusBar.update('idle');
        }
        catch (err) {
            statusBar.update('error');
            vscode.window.showErrorMessage(`Hydra: Failed to explain - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.fixError', async (diagnostic) => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('Hydra: No active editor');
            return;
        }
        let diagMessage;
        let code;
        if (diagnostic) {
            diagMessage = diagnostic.message;
            code = editor.document.getText(diagnostic.range);
        }
        else {
            const line = editor.selection.active.line;
            const allDiagnostics = vscode.languages.getDiagnostics(editor.document.uri);
            const lineDiag = allDiagnostics.find((d) => d.range.start.line === line);
            if (!lineDiag) {
                vscode.window.showWarningMessage('Hydra: No diagnostic on current line');
                return;
            }
            diagMessage = lineDiag.message;
            code = editor.document.lineAt(line).text;
        }
        try {
            statusBar.update('working', 'Fixing...');
            const fix = await client.fixError(code, diagMessage, editor.document.languageId);
            outputChannel.clear();
            outputChannel.appendLine('=== Hydra: Suggested Fix ===');
            outputChannel.appendLine('');
            outputChannel.appendLine(`Diagnostic: ${diagMessage}`);
            outputChannel.appendLine('');
            outputChannel.appendLine(fix);
            outputChannel.show(true);
            statusBar.update('idle');
        }
        catch (err) {
            statusBar.update('error');
            vscode.window.showErrorMessage(`Hydra: Failed to fix - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.generateTests', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('Hydra: No active editor');
            return;
        }
        const selection = editor.selection;
        const code = selection.isEmpty
            ? editor.document.getText()
            : editor.document.getText(selection);
        if (!code.trim()) {
            vscode.window.showWarningMessage('Hydra: No code to generate tests for');
            return;
        }
        try {
            statusBar.update('working', 'Generating tests...');
            const tests = await client.generateTests(code, editor.document.languageId);
            outputChannel.clear();
            outputChannel.appendLine('=== Hydra: Generated Tests ===');
            outputChannel.appendLine('');
            outputChannel.appendLine(tests);
            outputChannel.show(true);
            statusBar.update('idle');
        }
        catch (err) {
            statusBar.update('error');
            vscode.window.showErrorMessage(`Hydra: Failed to generate tests - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.showImpact', async (functionName, filePath) => {
        const editor = vscode.window.activeTextEditor;
        const name = functionName ?? getWordAtCursor(editor);
        const file = filePath ?? editor?.document.uri.fsPath;
        if (!name || !file) {
            vscode.window.showWarningMessage('Hydra: No function identified');
            return;
        }
        try {
            statusBar.update('working', 'Analyzing impact...');
            const impact = await client.getImpact(name, file);
            outputChannel.clear();
            outputChannel.appendLine(`=== Hydra: Impact Analysis — ${name} ===`);
            outputChannel.appendLine('');
            outputChannel.appendLine(`References: ${impact.references}`);
            outputChannel.appendLine('');
            outputChannel.appendLine(impact.details);
            outputChannel.show(true);
            statusBar.update('idle');
        }
        catch (err) {
            statusBar.update('error');
            vscode.window.showErrorMessage(`Hydra: Failed to analyze impact - ${err}`);
        }
    }), vscode.commands.registerCommand('hydra.suggestRefactor', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('Hydra: No active editor');
            return;
        }
        const selection = editor.selection;
        const code = selection.isEmpty
            ? editor.document.getText()
            : editor.document.getText(selection);
        if (!code.trim()) {
            vscode.window.showWarningMessage('Hydra: No code selected');
            return;
        }
        try {
            statusBar.update('working', 'Analyzing...');
            const suggestion = await client.suggestRefactor(code, editor.document.languageId);
            outputChannel.clear();
            outputChannel.appendLine('=== Hydra: Refactor Suggestion ===');
            outputChannel.appendLine('');
            outputChannel.appendLine(suggestion);
            outputChannel.show(true);
            statusBar.update('idle');
        }
        catch (err) {
            statusBar.update('error');
            vscode.window.showErrorMessage(`Hydra: Failed to suggest refactor - ${err}`);
        }
    }), outputChannel);
}
async function handleToggleVoice(client, statusBar) {
    try {
        const running = await client.isServerRunning();
        if (!running) {
            vscode.window.showWarningMessage('Hydra server is not running');
            return;
        }
        const result = await client.rpc('voice_toggle', {});
        const enabled = result?.enabled ?? false;
        vscode.window.showInformationMessage(enabled ? 'Voice enabled. Say "Hey Hydra"!' : 'Voice disabled');
    }
    catch (e) {
        vscode.window.showErrorMessage('Failed to toggle voice');
    }
}
function getWordAtCursor(editor) {
    if (!editor) {
        return undefined;
    }
    const position = editor.selection.active;
    const range = editor.document.getWordRangeAtPosition(position);
    return range ? editor.document.getText(range) : undefined;
}
//# sourceMappingURL=commands.js.map