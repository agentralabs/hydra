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

  updateBaseUrl(url: string): void {
    this.baseUrl = url.replace(/\/$/, '');
    this.protocol = this.baseUrl.startsWith('https') ? https : http;
  }

  async getStatus(): Promise<HydraStatus> {
    const data = await this.request('GET', '/api/status');
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
    await this.request('POST', '/rpc', {
      method: 'hydra.approve',
      params: { runId, actionId },
    });
  }

  async deny(runId: string, actionId: string): Promise<void> {
    await this.request('POST', '/rpc', {
      method: 'hydra.deny',
      params: { runId, actionId },
    });
  }

  async killAll(): Promise<void> {
    await this.request('POST', '/rpc', {
      method: 'hydra.kill',
      params: {},
    });
  }

  async getSisters(): Promise<SisterStatus[]> {
    const data = await this.request('GET', '/api/sisters');
    return data as SisterStatus[];
  }

  async getPendingApprovals(): Promise<PendingApproval[]> {
    const data = await this.request('GET', '/api/approvals/pending');
    return data as PendingApproval[];
  }

  async explainCode(code: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.explain',
      params: { code, languageId },
    });
    return (data as { explanation: string }).explanation;
  }

  async fixError(code: string, diagnostic: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.fixError',
      params: { code, diagnostic, languageId },
    });
    return (data as { fix: string }).fix;
  }

  async generateTests(code: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.generateTests',
      params: { code, languageId },
    });
    return (data as { tests: string }).tests;
  }

  async suggestRefactor(code: string, languageId: string): Promise<string> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.suggestRefactor',
      params: { code, languageId },
    });
    return (data as { suggestion: string }).suggestion;
  }

  async getImpact(
    functionName: string,
    filePath: string
  ): Promise<{ references: number; details: string }> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.impact',
      params: { functionName, filePath },
    });
    return data as { references: number; details: string };
  }

  async getDiagnostics(
    filePath: string,
    content: string,
    languageId: string
  ): Promise<Array<{ line: number; message: string; severity: string }>> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.diagnostics',
      params: { filePath, content, languageId },
    });
    return data as Array<{ line: number; message: string; severity: string }>;
  }

  async getHoverInfo(
    word: string,
    filePath: string,
    line: number,
    languageId: string
  ): Promise<{ explanation: string; references?: number } | null> {
    const data = await this.request('POST', '/rpc', {
      method: 'hydra.hover',
      params: { word, filePath, line, languageId },
    });
    return data as { explanation: string; references?: number } | null;
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
