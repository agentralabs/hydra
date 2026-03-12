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
exports.HydraHoverProvider = void 0;
exports.registerHoverProvider = registerHoverProvider;
const vscode = __importStar(require("vscode"));
class HydraHoverProvider {
    constructor(client) {
        this.client = client;
    }
    async provideHover(document, position, token) {
        const wordRange = document.getWordRangeAtPosition(position);
        if (!wordRange) {
            return null;
        }
        const word = document.getText(wordRange);
        if (!word || word.length < 2) {
            return null;
        }
        try {
            const running = await this.client.isServerRunning();
            if (!running) {
                return null;
            }
            if (token.isCancellationRequested) {
                return null;
            }
            const info = await this.client.getHoverInfo(word, document.uri.fsPath, position.line, document.languageId);
            if (!info) {
                return null;
            }
            const contents = new vscode.MarkdownString();
            contents.isTrusted = true;
            contents.appendMarkdown(`**Hydra** \u2014 \`${word}\`\n\n`);
            contents.appendMarkdown(info.explanation);
            if (info.references !== undefined) {
                contents.appendMarkdown(`\n\n---\n*${info.references} reference(s) across codebase*`);
            }
            return new vscode.Hover(contents, wordRange);
        }
        catch {
            // Server unavailable — return nothing
            return null;
        }
    }
}
exports.HydraHoverProvider = HydraHoverProvider;
function registerHoverProvider(context, client) {
    const provider = new HydraHoverProvider(client);
    context.subscriptions.push(vscode.languages.registerHoverProvider({ scheme: 'file' }, provider));
}
//# sourceMappingURL=hoverProvider.js.map