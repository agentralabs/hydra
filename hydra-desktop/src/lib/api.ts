import { DecisionResponse, Run } from '@/types/hydra';

const DEFAULT_BASE_URL = 'http://localhost:7777';

let rpcId = 0;

export class HydraAPI {
  private baseUrl: string;

  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl ?? DEFAULT_BASE_URL;
  }

  private async rpc<T>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    rpcId += 1;
    const res = await fetch(`${this.baseUrl}/rpc`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: rpcId,
        method,
        params,
      }),
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      throw new HydraAPIError(res.status, body || res.statusText);
    }

    const json = await res.json();

    if (json.error) {
      throw new HydraAPIError(json.error.code ?? 500, json.error.message ?? 'RPC error');
    }

    return json.result as T;
  }

  async health(): Promise<{ status: string; version: string }> {
    const res = await fetch(`${this.baseUrl}/health`);
    return res.json();
  }

  async sendMessage(content: string): Promise<{ run_id: string; status: string }> {
    return this.rpc<{ run_id: string; status: string }>('hydra.run', {
      intent: content,
    });
  }

  async submitDecision(response: DecisionResponse): Promise<void> {
    await this.rpc('hydra.approve', {
      approval_id: response.request_id,
      decision: response.chosen_option === 0 ? 'approved' : 'denied',
    });
  }

  async getRun(runId: string): Promise<Run> {
    const result = await this.rpc<{ runs: Run[] }>('hydra.status', { run_id: runId });
    if (result.runs.length === 0) {
      throw new HydraAPIError(404, 'Run not found');
    }
    return result.runs[0];
  }

  async cancelRun(runId: string): Promise<void> {
    await this.rpc('hydra.cancel', { run_id: runId });
  }

  async getStatus(): Promise<{ runs: Run[] }> {
    return this.rpc('hydra.status', {});
  }

  async kill(level: 'instant' | 'graceful' | 'freeze', reason?: string): Promise<void> {
    await this.rpc('hydra.kill', { level, reason });
  }
}

export class HydraAPIError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
  ) {
    super(`Hydra API error (${status}): ${body}`);
    this.name = 'HydraAPIError';
  }
}
