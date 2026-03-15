import * as vscode from 'vscode';

/**
 * Shows a blue dot badge on files that have been edited by Hydra during
 * the current session. Tracked via change events from the server.
 */
export class HydraFileDecorationProvider
  implements vscode.FileDecorationProvider
{
  private _onDidChangeFileDecorations = new vscode.EventEmitter<
    vscode.Uri | vscode.Uri[] | undefined
  >();
  readonly onDidChangeFileDecorations = this._onDidChangeFileDecorations.event;

  /** Set of absolute file paths modified by Hydra. */
  private modifiedFiles: Set<string> = new Set();

  provideFileDecoration(
    uri: vscode.Uri,
    _token: vscode.CancellationToken
  ): vscode.ProviderResult<vscode.FileDecoration> {
    if (!this.modifiedFiles.has(uri.fsPath)) {
      return undefined;
    }

    return {
      badge: 'H',
      color: new vscode.ThemeColor('charts.blue'),
      tooltip: 'Modified by Hydra',
      propagate: false,
    };
  }

  /** Mark a file as modified by Hydra. */
  markModified(filePath: string): void {
    const uri = vscode.Uri.file(filePath);
    this.modifiedFiles.add(uri.fsPath);
    this._onDidChangeFileDecorations.fire(uri);
  }

  /** Clear all decorations (e.g., on session reset). */
  clearAll(): void {
    const uris = Array.from(this.modifiedFiles).map((p) => vscode.Uri.file(p));
    this.modifiedFiles.clear();
    this._onDidChangeFileDecorations.fire(uris);
  }

  dispose(): void {
    this._onDidChangeFileDecorations.dispose();
  }
}

export function registerFileDecorationProvider(
  context: vscode.ExtensionContext
): HydraFileDecorationProvider {
  const provider = new HydraFileDecorationProvider();
  context.subscriptions.push(
    vscode.window.registerFileDecorationProvider(provider)
  );
  context.subscriptions.push(provider);
  return provider;
}
