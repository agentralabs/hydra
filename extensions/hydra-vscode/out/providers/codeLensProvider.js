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
exports.HydraCodeLensProvider = void 0;
exports.registerCodeLensProvider = registerCodeLensProvider;
const vscode = __importStar(require("vscode"));
/** Regex matching common function/method declarations across languages. */
const FUNCTION_PATTERN = /^[ \t]*(?:export\s+)?(?:async\s+)?(?:function\s+\w+|(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+\w+|def\s+\w+|(?:public|private|protected|static|\s)*\s+\w+\s*\([^)]*\)\s*(?:\{|:)|(?:const|let|var)\s+\w+\s*=\s*(?:async\s+)?\()/;
class HydraCodeLensProvider {
    constructor(client) {
        this.client = client;
        this._onDidChangeCodeLenses = new vscode.EventEmitter();
        this.onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;
    }
    async provideCodeLenses(document, token) {
        const lenses = [];
        for (let i = 0; i < document.lineCount; i++) {
            if (token.isCancellationRequested) {
                return lenses;
            }
            const line = document.lineAt(i);
            if (!FUNCTION_PATTERN.test(line.text)) {
                continue;
            }
            const range = new vscode.Range(i, 0, i, line.text.length);
            const functionName = extractFunctionName(line.text);
            if (!functionName) {
                continue;
            }
            // Impact analysis lens
            lenses.push(new vscode.CodeLens(range, {
                title: '$(references) Hydra: Impact Analysis',
                command: 'hydra.showImpact',
                arguments: [functionName, document.uri.fsPath],
                tooltip: 'Show impact analysis for this function',
            }));
            // Explain lens for longer functions
            const bodyLength = estimateFunctionLength(document, i);
            if (bodyLength > 10) {
                lenses.push(new vscode.CodeLens(range, {
                    title: '$(lightbulb) Hydra: Explain',
                    command: 'hydra.explain',
                    tooltip: 'Explain this function',
                }));
            }
            // Complexity indicator for long functions
            const funcEnd = this.findFunctionEnd(document, i);
            const lineCount = funcEnd - i;
            if (lineCount > 20) {
                lenses.push(new vscode.CodeLens(range, {
                    title: `\u26a0\ufe0f Complexity: ${lineCount} lines`,
                    command: 'hydra.suggestRefactor',
                    arguments: [document.getText(new vscode.Range(range.start, new vscode.Position(funcEnd, 0))), document.uri, range],
                }));
            }
        }
        // Resolve references in background if server is available
        this.resolveReferences(document, lenses);
        return lenses;
    }
    async resolveReferences(document, lenses) {
        try {
            const running = await this.client.isServerRunning();
            if (!running) {
                return;
            }
            for (const lens of lenses) {
                if (!lens.command || !lens.command.command.includes('showImpact')) {
                    continue;
                }
                const [functionName, filePath] = lens.command.arguments;
                try {
                    const impact = await this.client.getImpact(functionName, filePath);
                    lens.command.title = `$(references) Hydra: Impact Analysis (${impact.references} references)`;
                }
                catch {
                    // Keep default title without reference count
                }
            }
            this._onDidChangeCodeLenses.fire();
        }
        catch {
            // Server unavailable — leave default titles
        }
    }
    findFunctionEnd(document, startLine) {
        let braceCount = 0;
        let started = false;
        for (let i = startLine; i < document.lineCount; i++) {
            const line = document.lineAt(i).text;
            for (const ch of line) {
                if (ch === '{') {
                    braceCount++;
                    started = true;
                }
                if (ch === '}') {
                    braceCount--;
                }
                if (started && braceCount === 0) {
                    return i;
                }
            }
        }
        return Math.min(startLine + 50, document.lineCount);
    }
    refresh() {
        this._onDidChangeCodeLenses.fire();
    }
}
exports.HydraCodeLensProvider = HydraCodeLensProvider;
function extractFunctionName(lineText) {
    // Match: function foo, fn foo, def foo, const foo =, let foo =
    const patterns = [
        /function\s+(\w+)/,
        /fn\s+(\w+)/,
        /def\s+(\w+)/,
        /(?:const|let|var)\s+(\w+)\s*=/,
        /(\w+)\s*\([^)]*\)\s*(?:\{|:)/,
    ];
    for (const pattern of patterns) {
        const match = lineText.match(pattern);
        if (match) {
            return match[1];
        }
    }
    return null;
}
function estimateFunctionLength(document, startLine) {
    let depth = 0;
    let started = false;
    for (let i = startLine; i < document.lineCount; i++) {
        const text = document.lineAt(i).text;
        for (const ch of text) {
            if (ch === '{') {
                depth++;
                started = true;
            }
            else if (ch === '}') {
                depth--;
            }
        }
        if (started && depth <= 0) {
            return i - startLine;
        }
    }
    // Fallback: count indented lines for Python-style
    let count = 0;
    if (startLine + 1 < document.lineCount) {
        const baseIndent = document.lineAt(startLine).firstNonWhitespaceCharacterIndex;
        for (let i = startLine + 1; i < document.lineCount; i++) {
            const line = document.lineAt(i);
            if (line.isEmptyOrWhitespace) {
                count++;
                continue;
            }
            if (line.firstNonWhitespaceCharacterIndex > baseIndent) {
                count++;
            }
            else {
                break;
            }
        }
    }
    return count;
}
function registerCodeLensProvider(context, client) {
    const provider = new HydraCodeLensProvider(client);
    context.subscriptions.push(vscode.languages.registerCodeLensProvider({ scheme: 'file' }, provider));
    return provider;
}
//# sourceMappingURL=codeLensProvider.js.map