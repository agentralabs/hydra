import * as vscode from 'vscode';
import { HydraClient } from '../hydraClient';

export class HydraCodeActionsProvider implements vscode.CodeActionProvider {
  public static readonly providedCodeActionKinds = [
    vscode.CodeActionKind.QuickFix,
    vscode.CodeActionKind.Refactor,
  ];

  constructor(private readonly client: HydraClient) {}

  provideCodeActions(
    document: vscode.TextDocument,
    range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext,
    _token: vscode.CancellationToken
  ): vscode.CodeAction[] {
    const actions: vscode.CodeAction[] = [];
    const hasSelection = !range.isEmpty;
    const hasDiagnostics = context.diagnostics.length > 0;

    if (hasSelection) {
      const explainAction = new vscode.CodeAction(
        'Hydra: Explain this code',
        vscode.CodeActionKind.QuickFix
      );
      explainAction.command = {
        command: 'hydra.explain',
        title: 'Hydra: Explain this code',
      };
      actions.push(explainAction);

      const testAction = new vscode.CodeAction(
        'Hydra: Generate tests',
        vscode.CodeActionKind.QuickFix
      );
      testAction.command = {
        command: 'hydra.generateTests',
        title: 'Hydra: Generate tests',
      };
      actions.push(testAction);

      const refactorAction = new vscode.CodeAction(
        'Hydra: Suggest refactor',
        vscode.CodeActionKind.Refactor
      );
      refactorAction.command = {
        command: 'hydra.suggestRefactor',
        title: 'Hydra: Suggest refactor',
      };
      actions.push(refactorAction);
    }

    if (hasDiagnostics) {
      for (const diagnostic of context.diagnostics) {
        const fixAction = new vscode.CodeAction(
          `Hydra: Fix - ${diagnostic.message.slice(0, 60)}`,
          vscode.CodeActionKind.QuickFix
        );
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

export function registerCodeActionsProvider(
  context: vscode.ExtensionContext,
  client: HydraClient
): void {
  const provider = new HydraCodeActionsProvider(client);
  context.subscriptions.push(
    vscode.languages.registerCodeActionsProvider(
      { scheme: 'file' },
      provider,
      { providedCodeActionKinds: HydraCodeActionsProvider.providedCodeActionKinds }
    )
  );
}
