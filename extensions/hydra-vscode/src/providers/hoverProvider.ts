import * as vscode from 'vscode';
import { HydraClient } from '../hydraClient';

export class HydraHoverProvider implements vscode.HoverProvider {
  constructor(private readonly client: HydraClient) {}

  async provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken
  ): Promise<vscode.Hover | null> {
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

      const info = await this.client.getHoverInfo(
        word,
        document.uri.fsPath,
        position.line,
        document.languageId
      );

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
    } catch {
      // Server unavailable — return nothing
      return null;
    }
  }
}

export function registerHoverProvider(
  context: vscode.ExtensionContext,
  client: HydraClient
): void {
  const provider = new HydraHoverProvider(client);
  context.subscriptions.push(
    vscode.languages.registerHoverProvider(
      { scheme: 'file' },
      provider
    )
  );
}
