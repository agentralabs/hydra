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
exports.HydraClient = void 0;
const http = __importStar(require("http"));
const https = __importStar(require("https"));
class HydraClient {
    constructor(baseUrl = 'http://localhost:7777') {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.protocol = this.baseUrl.startsWith('https') ? https : http;
    }
    updateBaseUrl(url) {
        this.baseUrl = url.replace(/\/$/, '');
        this.protocol = this.baseUrl.startsWith('https') ? https : http;
    }
    async getStatus() {
        const data = await this.request('GET', '/api/system/status');
        return data;
    }
    async createRun(intent) {
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent },
        });
        return data;
    }
    async getActiveRuns() {
        const data = await this.request('GET', '/api/runs?status=running');
        return data;
    }
    async approve(runId, actionId) {
        await this.request('POST', `/api/approvals/${actionId}/approve`);
    }
    async deny(runId, actionId) {
        await this.request('POST', `/api/approvals/${actionId}/deny`, {
            reason: 'Denied via VS Code extension',
        });
    }
    async killAll() {
        await this.request('POST', '/rpc', {
            method: 'hydra.kill',
            params: {},
        });
    }
    async getSisters() {
        const status = await this.request('GET', '/api/system/status');
        const sisters = status?.sisters || {};
        return Object.entries(sisters).map(([name, state]) => ({
            name,
            connected: state === 'connected',
            error: state === 'connected' ? undefined : String(state),
        }));
    }
    async getPendingApprovals() {
        const data = await this.request('GET', '/api/approvals');
        return data.filter((a) => a.status === 'pending');
    }
    async getInventions() {
        return await this.request('GET', '/api/system/inventions');
    }
    async getTrust() {
        return await this.request('GET', '/api/system/trust');
    }
    async getBudget() {
        return await this.request('GET', '/api/system/budget');
    }
    async explainCode(code, languageId) {
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Explain this ${languageId} code:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
        });
        return data?.content || 'Explanation pending...';
    }
    async fixError(code, diagnostic, languageId) {
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Fix this ${languageId} error: "${diagnostic}" in:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
        });
        return data?.content || 'Fix pending...';
    }
    async generateTests(code, languageId) {
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Generate tests for this ${languageId} code:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
        });
        return data?.content || 'Tests pending...';
    }
    async suggestRefactor(code, languageId) {
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Suggest refactoring for this ${languageId} code:\n\`\`\`${languageId}\n${code}\n\`\`\`` },
        });
        return data?.content || 'Suggestion pending...';
    }
    async getImpact(functionName, filePath) {
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Analyze impact of changing function "${functionName}" in ${filePath}` },
        });
        return { references: 0, details: data?.content || 'Impact analysis pending...' };
    }
    async getDiagnostics(filePath, content, languageId) {
        // Diagnostics run through the cognitive loop as a hydra.run intent
        await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Analyze ${languageId} file ${filePath} for issues` },
        });
        return [];
    }
    async getHoverInfo(word, filePath, line, languageId) {
        // Hover info dispatched as a lightweight run
        const data = await this.request('POST', '/rpc', {
            method: 'hydra.run',
            params: { intent: `Explain "${word}" at line ${line} in ${filePath} (${languageId})` },
        });
        return { explanation: data?.content || '' };
    }
    async rpc(method, params) {
        const data = await this.request('POST', '/rpc', { method: `hydra.${method}`, params });
        return data;
    }
    async isServerRunning() {
        try {
            await this.getStatus();
            return true;
        }
        catch {
            return false;
        }
    }
    request(method, path, body) {
        return new Promise((resolve, reject) => {
            const url = new URL(path, this.baseUrl);
            const bodyStr = body ? JSON.stringify(body) : undefined;
            const options = {
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
                res.on('data', (chunk) => { data += chunk.toString(); });
                res.on('end', () => {
                    if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
                        try {
                            resolve(data ? JSON.parse(data) : undefined);
                        }
                        catch {
                            resolve(data);
                        }
                    }
                    else {
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
exports.HydraClient = HydraClient;
//# sourceMappingURL=hydraClient.js.map