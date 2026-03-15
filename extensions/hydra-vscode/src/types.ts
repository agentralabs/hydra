export interface HydraStatus {
  connected: boolean;
  activeRuns: number;
  pendingApprovals: number;
  phase: string | null;
  tokensToday: number;
  // Stats for status bar (Pattern 19 + Hydra Pattern 5)
  sistersConnected?: number;
  sistersTotal?: number;
  activeProfile?: string;
  beliefsLoaded?: number;
  sessionCost?: number;
  compiledPatterns?: number;
  compiledSavings?: number;
  thinkingVerb?: string;
  briefingItems?: Array<{ priority: string; text: string }>;
}

export interface Run {
  id: string;
  intent: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'paused';
  createdAt: string;
  completedAt?: string;
  tokensUsed: number;
}

export interface PendingApproval {
  id: string;
  runId: string;
  action: string;
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
  summary: string;
  timeoutSecs: number;
}

export interface SisterStatus {
  name: string;
  connected: boolean;
  error?: string;
}

export type IconState = 'idle' | 'listening' | 'working' | 'needsAttention' | 'approvalNeeded' | 'success' | 'error' | 'offline';
