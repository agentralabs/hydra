import * as http from 'http';
import * as https from 'https';
import { HydraStatus, Run, PendingApproval, SisterStatus } from './types';

export class HydraClient {
  private baseUrl: string;
  private protocol: typeof http | typeof https;

  constructor(baseUrl: string = 'http://localhost:7777') {
    this.baseUrl = baseUrl.replace(/\/$/, '');
    this.protocol = this.baseUrl.startsWith('https') ? https : http;
  }

  getBaseUrl(): string {
    return this.baseUrl;
  }

  updateBaseUrl(url: string): void {
    this.baseUrl = url.replace(/\/$/, '');
    this.protocol = this.baseUrl.startsWith('https') ? https : http;
  }

  async getStatus(): Promise<HydraStatus> {
    const data = await this.request('GET', '/api/system/status');
    return data as HydraStatus;
  }

  async createRun(intent: string): Promise<Run> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent },
    });
    return data as Run;
  }

  async getActiveRuns(): Promise<Run[]> {
    const data = await this.request('GET', '/api/runs?status=running');
    return data as Run[];
  }

  async approve(runId: string, actionId: string): Promise<void> {
    await this.request('POST', `/api/approvals/${actionId}/approve`);
  }

  async deny(runId: string, actionId: string): Promise<void> {
    await this.request('POST', `/api/approvals/${actionId}/deny`, {
      reason: 'Denied via VS Code extension',
    });
  }

  async killAll(): Promise<void> {
    await this.request('POST', '/rpc', {
      method: 'hydra.kill',
      params: {},
    });
  }

  async getSisters(): Promise<SisterStatus[]> {
    const status = await this.request('GET', '/api/system/status') as any;
    const sisters = status?.sisters || {};
    return Object.entries(sisters).map(([name, state]) => ({
      name,
      connected: state === 'connected',
      error: state === 'connected' ? undefined : String(state),
    }));
  }

  async getPendingApprovals(): Promise<PendingApproval[]> {
    const data = await this.request('GET', '/api/approvals');
    return (data as any[]).filter((a: any) => a.status === 'pending') as PendingApproval[];
  }

  async getInventions(): Promise<Record<string, unknown>> {
    return await this.request('GET', '/api/system/inventions') as Record<string, unknown>;
  }

  async getTrust(): Promise<{ trust_score: number; autonomy_level: string }> {
    return await this.request('GET', '/api/system/trust') as any;
  }

  async getBudget(): Promise<Record<string, unknown>> {
    return await this.request('GET', '/api/system/budget') as Record<string, unknown>;
  }

  async explainCode(code: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Explain this ${languageId} code:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
    });
    return (data as any)?.content || 'Explanation pending...';
  }

  async fixError(code: string, diagnostic: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Fix this ${languageId} error: "${diagnostic}" in:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
    });
    return (data as any)?.content || 'Fix pending...';
  }

  async generateTests(code: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Generate tests for this ${languageId} code:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
    });
    return (data as any)?.content || 'Tests pending...';
  }

  async suggestRefactor(code: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Suggest refactoring for this ${languageId} code:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
    });
    return (data as any)?.content || 'Suggestion pending...';
  }

  async getImpact(
    functionName: string,
    filePath: string
  ): Promise<{ references: number; details: string }> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Analyze impact of changing function "${functionName}" in ${filePath}` },
    });
    return { references: 0, details: (data as any)?.content || 'Impact analysis pending...' };
  }

  async getDiagnostics(
    filePath: string,
    content: string,
    languageId: string
  ): Promise<Array<{ line: number; message: string; severity: string }>> {
    // Diagnostics run through the cognitive loop as a hydra.run intent
    await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Analyze ${languageId} file ${filePath} for issues` },
    });
    return [];
  }

  async getHoverInfo(
    word: string,
    filePath: string,
    line: number,
    languageId: string
  ): Promise<{ explanation: string; references?: number } | null> {
    // Hover info dispatched as a lightweight run
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.run',
      params: { intent: `Explain "${word}" at line ${line} in ${filePath} (${languageId})` },
    });
    return { explanation: (data as any)?.content || '' };
  }

  async rpc(method: string, params: Record<string, unknown>): Promise<any> {
    const data = await this.request('POST', '/rpc', { method: `hydra.${method}`, params });
    return data;
  }

  async isServerRunning(): Promise<boolean> {
    try {
      await this.getStatus();
      return true;
    } catch {
      return false;
    }
  }

  private request(method: string, path: string, body?: unknown): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const url = new URL(path, this.baseUrl);
      const bodyStr = body ? JSON.stringify(body) : undefined;

      const options: http.RequestOptions = {
        hostname: url.hostname,
        port: url.port,
        path: url.pathname + url.search,
        method,
        headers: {
          'Accept': 'application/json',
          ...(bodyStr ? {
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(bodyStr),
          } : {}),
        },
        timeout: 5000,
      };

      const req = this.protocol.request(options, (res) => {
        let data = '';
        res.on('data', (chunk: Buffer) => { data += chunk.toString(); });
        res.on('end', () => {
          if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            try {
              resolve(data ? JSON.parse(data) : undefined);
            } catch {
              resolve(data);
            }
          } else {
            reject(new Error(`HTTP ${res.statusCode}: ${data}`));
          }
        });
      });

      req.on('error', (err) => reject(err));
      req.on('timeout', () => {
        req.destroy();
        reject(new Error('Request timed out'));
      });

      if (bodyStr) {
        req.write(bodyStr);
      }
      req.end();
    });
  }
}
