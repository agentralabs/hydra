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
exports.HydraCodeActionsProvider = void 0;
exports.registerCodeActionsProvider = registerCodeActionsProvider;
const vscode = __importStar(require("vscode"));
class HydraCodeActionsProvider {
    constructor(client) {
        this.client = client;
    }
    provideCodeActions(document, range, context, _token) {
        const actions = [];
        const hasSelection = !range.isEmpty;
        const hasDiagnostics = context.diagnostics.length > 0;
        if (hasSelection) {
            const explainAction = new vscode.CodeAction('Hydra: Explain this code', vscode.CodeActionKind.QuickFix);
            explainAction.command = {
                command: 'hydra.explain',
                title: 'Hydra: Explain this code',
            };
            actions.push(explainAction);
            const testAction = new vscode.CodeAction('Hydra: Generate tests', vscode.CodeActionKind.QuickFix);
            testAction.command = {
                command: 'hydra.generateTests',
                title: 'Hydra: Generate tests',
            };
            actions.push(testAction);
            const refactorAction = new vscode.CodeAction('Hydra: Suggest refactor', vscode.CodeActionKind.Refactor);
            refactorAction.command = {
                command: 'hydra.suggestRefactor',
                title: 'Hydra: Suggest refactor',
            };
            actions.push(refactorAction);
        }
        if (hasDiagnostics) {
            for (const diagnostic of context.diagnostics) {
                const fixAction = new vscode.CodeAction(`Hydra: Fix - ${diagnostic.message.slice(0, 60)}`, vscode.CodeActionKind.QuickFix);
                fixAction.command = {
                    command: 'hydra.fixError',
                    title: 'Hydra: Fix this error',
                    arguments: [diagnostic],
                };
                fixAction.diagnostics = [diagnostic];
                actions.push(fixAction);
            }
        }
        return actions;
    }
}
exports.HydraCodeActionsProvider = HydraCodeActionsProvider;
HydraCodeActionsProvider.providedCodeActionKinds = [
    vscode.CodeActionKind.QuickFix,
    vscode.CodeActionKind.Refactor,
];
function registerCodeActionsProvider(context, client) {
    const provider = new HydraCodeActionsProvider(client);
    context.subscriptions.push(vscode.languages.registerCodeActionsProvider({ scheme: 'file' }, provider, { providedCodeActionKinds: HydraCodeActionsProvider.providedCodeActionKinds }));
}
//# sourceMappingURL=codeActionsProvider.js.map