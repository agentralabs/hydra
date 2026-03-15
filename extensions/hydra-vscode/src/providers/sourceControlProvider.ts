import * as vscode from 'vscode';
import { HydraClient } from '../hydraClient';

/**
 * Provides a Source Control view for Hydra session file changes.
 * When the server reports file changes (via polling or SSE), they appear
 * as source control resources under the "Hydra Session Changes" group.
 */
export class HydraSourceControlProvider implements vscode.Disposable {
  private scm: vscode.SourceControl;
  private changesGroup: vscode.SourceControlResourceGroup;
  private trackedFiles: Map<string, vscode.SourceControlResourceState> = new Map();
  private disposables: vscode.Disposable[] = [];

  constructor(
    private readonly client: HydraClient
  ) {
    this.scm = vscode.scm.createSourceControl('hydra', 'Hydra');
    this.scm.acceptInputCommand = {
      command: 'hydra.commitChanges',
      title: 'Commit Hydra Changes',
    };
    this.scm.inputBox.placeholder = 'Describe changes made by Hydra...';

    this.changesGroup = this.scm.createResourceGroup('hydra-changes', 'Hydra Session Changes');
    this.changesGroup.hideWhenEmpty = true;

    this.disposables.push(this.scm);
  }

  /**
   * Called when the server reports a file change (e.g., from SSE events).
   * Adds or updates the file in the source control resource group.
   */
  addFileChange(filePath: string, changeType: 'created' | 'edited' | 'deleted'): void {
    const uri = vscode.Uri.file(filePath);
    const decoration = this.decorationForChange(changeType);

    const resource: vscode.SourceControlResourceState = {
      resourceUri: uri,
      decorations: {
        strikeThrough: changeType === 'deleted',
        tooltip: `Hydra: ${changeType}`,
        iconPath: decoration,
      },
    };

    this.trackedFiles.set(filePath, resource);
    this.changesGroup.resourceStates = Array.from(this.trackedFiles.values());
    this.scm.count = this.trackedFiles.size;
  }

  /** Clear all tracked changes (e.g., after commit). */
  clearChanges(): void {
    this.trackedFiles.clear();
    this.changesGroup.resourceStates = [];
    this.scm.count = 0;
  }

  private decorationForChange(
    changeType: string
  ): vscode.ThemeIcon {
    switch (changeType) {
      case 'created':
        return new vscode.ThemeIcon('diff-added');
      case 'deleted':
        return new vscode.ThemeIcon('diff-removed');
      default:
        return new vscode.ThemeIcon('diff-modified');
    }
  }

  dispose(): void {
    for (const d of this.disposables) {
      d.dispose();
    }
  }
}

export function registerSourceControlProvider(
  context: vscode.ExtensionContext,
  client: HydraClient
): HydraSourceControlProvider {
  const provider = new HydraSourceControlProvider(client);

  context.subscriptions.push(provider);

  context.subscriptions.push(
    vscode.commands.registerCommand('hydra.commitChanges', async () => {
      const message = provider['scm'].inputBox.value;
      if (!message) {
        vscode.window.showWarningMessage('Hydra: Please enter a commit message');
        return;
      }
      try {
        await client.rpc('commit', { message });
        provider.clearChanges();
        provider['scm'].inputBox.value = '';
        vscode.window.showInformationMessage('Hydra: Changes committed');
      } catch (err) {
        vscode.window.showErrorMessage(`Hydra: Commit failed - ${err}`);
      }
    })
  );

  return provider;
}
