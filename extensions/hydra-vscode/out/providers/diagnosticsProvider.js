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
exports.HydraDiagnosticsProvider = void 0;
exports.registerDiagnosticsProvider = registerDiagnosticsProvider;
const vscode = __importStar(require("vscode"));
class HydraDiagnosticsProvider {
    constructor(client) {
        this.client = client;
        this.disposables = [];
        this.collection = vscode.languages.createDiagnosticCollection('hydra');
        this.disposables.push(vscode.workspace.onDidSaveTextDocument((doc) => {
            this.updateDiagnostics(doc);
        }));
        this.disposables.push(vscode.workspace.onDidCloseTextDocument((doc) => {
            this.collection.delete(doc.uri);
        }));
        // Run on all currently open documents
        for (const editor of vscode.window.visibleTextEditors) {
            this.updateDiagnostics(editor.document);
        }
    }
    async updateDiagnostics(document) {
        if (document.uri.scheme !== 'file') {
            return;
        }
        try {
            const running = await this.client.isServerRunning();
            if (!running) {
                this.collection.delete(document.uri);
                return;
            }
            const items = await this.client.getDiagnostics(document.uri.fsPath, document.getText(), document.languageId);
            const diagnostics = items.map((item) => {
                const line = Math.max(0, Math.min(item.line, document.lineCount - 1));
                const range = document.lineAt(line).range;
                const severity = mapSeverity(item.severity);
                const diagnostic = new vscode.Diagnostic(range, `Hydra: ${item.message}`, severity);
                diagnostic.source = 'Hydra';
                return diagnostic;
            });
            this.collection.set(document.uri, diagnostics);
        }
        catch {
            // Server unavailable — clear diagnostics silently
            this.collection.delete(document.uri);
        }
    }
    dispose() {
        this.collection.dispose();
        for (const d of this.disposables) {
            d.dispose();
        }
    }
}
exports.HydraDiagnosticsProvider = HydraDiagnosticsProvider;
function mapSeverity(severity) {
    switch (severity) {
        case 'error':
            return vscode.DiagnosticSeverity.Error;
        case 'warning':
            return vscode.DiagnosticSeverity.Warning;
        case 'info':
            return vscode.DiagnosticSeverity.Information;
        case 'hint':
            return vscode.DiagnosticSeverity.Hint;
        default:
            return vscode.DiagnosticSeverity.Warning;
    }
}
function registerDiagnosticsProvider(context, client) {
    const provider = new HydraDiagnosticsProvider(client);
    context.subscriptions.push(provider);
    return provider;
}
//# sourceMappingURL=diagnosticsProvider.js.map