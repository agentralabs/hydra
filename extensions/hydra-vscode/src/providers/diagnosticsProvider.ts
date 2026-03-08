import * as vscode from 'vscode';
import { HydraClient } from '../hydraClient';

export class HydraDiagnosticsProvider implements vscode.Disposable {
  private readonly collection: vscode.DiagnosticCollection;
  private readonly disposables: vscode.Disposable[] = [];

  constructor(private readonly client: HydraClient) {
    this.collection = vscode.languages.createDiagnosticCollection('hydra');

    this.disposables.push(
      vscode.workspace.onDidSaveTextDocument((doc) => {
        this.updateDiagnostics(doc);
      })
    );

    this.disposables.push(
      vscode.workspace.onDidCloseTextDocument((doc) => {
        this.collection.delete(doc.uri);
      })
    );

    // Run on all currently open documents
    for (const editor of vscode.window.visibleTextEditors) {
      this.updateDiagnostics(editor.document);
    }
  }

  async updateDiagnostics(document: vscode.TextDocument): Promise<void> {
    if (document.uri.scheme !== 'file') {
      return;
    }

    try {
      const running = await this.client.isServerRunning();
      if (!running) {
        this.collection.delete(document.uri);
        return;
      }

      const items = await this.client.getDiagnostics(
        document.uri.fsPath,
        document.getText(),
        document.languageId
      );

      const diagnostics: vscode.Diagnostic[] = items.map((item) => {
        const line = Math.max(0, Math.min(item.line, document.lineCount - 1));
        const range = document.lineAt(line).range;
        const severity = mapSeverity(item.severity);

        const diagnostic = new vscode.Diagnostic(
          range,
          `Hydra: ${item.message}`,
          severity
        );
        diagnostic.source = 'Hydra';
        return diagnostic;
      });

      this.collection.set(document.uri, diagnostics);
    } catch {
      // Server unavailable — clear diagnostics silently
      this.collection.delete(document.uri);
    }
  }

  dispose(): void {
    this.collection.dispose();
    for (const d of this.disposables) {
      d.dispose();
    }
  }
}

function mapSeverity(severity: string): vscode.DiagnosticSeverity {
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

export function registerDiagnosticsProvider(
  context: vscode.ExtensionContext,
  client: HydraClient
): HydraDiagnosticsProvider {
  const provider = new HydraDiagnosticsProvider(client);
  context.subscriptions.push(provider);
  return provider;
}
