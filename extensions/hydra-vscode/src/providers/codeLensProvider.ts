import * as vscode from 'vscode';
import { HydraClient } from '../hydraClient';

/** Regex matching common function/method declarations across languages. */
const FUNCTION_PATTERN =
  /^[ \t]*(?:export\s+)?(?:async\s+)?(?:function\s+\w+|(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+\w+|def\s+\w+|(?:public|private|protected|static|\s)*\s+\w+\s*\([^)]*\)\s*(?:\{|:)|(?:const|let|var)\s+\w+\s*=\s*(?:async\s+)?\()/;

export class HydraCodeLensProvider implements vscode.CodeLensProvider {
  private _onDidChangeCodeLenses = new vscode.EventEmitter<void>();
  public readonly onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;

  constructor(private readonly client: HydraClient) {}

  async provideCodeLenses(
    document: vscode.TextDocument,
    token: vscode.CancellationToken
  ): Promise<vscode.CodeLens[]> {
    const lenses: vscode.CodeLens[] = [];

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
      lenses.push(
        new vscode.CodeLens(range, {
          title: '$(references) Hydra: Impact Analysis',
          command: 'hydra.showImpact',
          arguments: [functionName, document.uri.fsPath],
          tooltip: 'Show impact analysis for this function',
        })
      );

      // Explain lens for longer functions
      const bodyLength = estimateFunctionLength(document, i);
      if (bodyLength > 10) {
        lenses.push(
          new vscode.CodeLens(range, {
            title: '$(lightbulb) Hydra: Explain',
            command: 'hydra.explain',
            tooltip: 'Explain this function',
          })
        );
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

  private async resolveReferences(
    document: vscode.TextDocument,
    lenses: vscode.CodeLens[]
  ): Promise<void> {
    try {
      const running = await this.client.isServerRunning();
      if (!running) {
        return;
      }

      for (const lens of lenses) {
        if (!lens.command || !lens.command.command.includes('showImpact')) {
          continue;
        }
        const [functionName, filePath] = lens.command.arguments as [string, string];
        try {
          const impact = await this.client.getImpact(functionName, filePath);
          lens.command.title = `$(references) Hydra: Impact Analysis (${impact.references} references)`;
        } catch {
          // Keep default title without reference count
        }
      }

      this._onDidChangeCodeLenses.fire();
    } catch {
      // Server unavailable — leave default titles
    }
  }

  private findFunctionEnd(document: vscode.TextDocument, startLine: number): number {
    let braceCount = 0;
    let started = false;
    for (let i = startLine; i < document.lineCount; i++) {
      const line = document.lineAt(i).text;
      for (const ch of line) {
        if (ch === '{') { braceCount++; started = true; }
        if (ch === '}') { braceCount--; }
        if (started && braceCount === 0) { return i; }
      }
    }
    return Math.min(startLine + 50, document.lineCount);
  }

  refresh(): void {
    this._onDidChangeCodeLenses.fire();
  }
}

function extractFunctionName(lineText: string): string | null {
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

function estimateFunctionLength(
  document: vscode.TextDocument,
  startLine: number
): number {
  let depth = 0;
  let started = false;

  for (let i = startLine; i < document.lineCount; i++) {
    const text = document.lineAt(i).text;

    for (const ch of text) {
      if (ch === '{') {
        depth++;
        started = true;
      } else if (ch === '}') {
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
      } else {
        break;
      }
    }
  }

  return count;
}

export function registerCodeLensProvider(
  context: vscode.ExtensionContext,
  client: HydraClient
): HydraCodeLensProvider {
  const provider = new HydraCodeLensProvider(client);
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider(
      { scheme: 'file' },
      provider
    )
  );
  return provider;
}
