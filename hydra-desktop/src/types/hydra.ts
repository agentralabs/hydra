// Mirror of hydra-core Rust types for the desktop frontend

export type IconState =
  | 'idle'
  | 'listening'
  | 'working'
  | 'needs_attention'
  | 'approval_needed'
  | 'success'
  | 'error'
  | 'offline';

export type RiskLevel = 'none' | 'low' | 'medium' | 'high' | 'critical';

export type AlertLevel = 'info' | 'warning' | 'error';

export type SseEventType =
  | 'run_started'
  | 'step_started'
  | 'step_progress'
  | 'step_completed'
  | 'approval_required'
  | 'run_completed'
  | 'run_error'
  | 'heartbeat'
  | 'system_ready'
  | 'system_shutdown';

export interface SseEvent {
  event_type: SseEventType;
  data: unknown;
  timestamp: string;
}

export interface DecisionOption {
  label: string;
  description?: string;
  risk_level?: RiskLevel;
  keyboard_shortcut?: string;
}

export interface DecisionRequest {
  id: string;
  question: string;
  options: DecisionOption[];
  timeout_seconds?: number;
  default?: number;
}

export interface DecisionResponse {
  request_id: string;
  chosen_option: number;
  custom_input?: string;
}

export interface CompletionSummary {
  headline: string;
  actions: string[];
  changes: string[];
  next_steps: string[];
}

export interface ProactiveUpdate {
  type: 'acknowledgment' | 'progress' | 'event' | 'decision' | 'completion' | 'alert';
  message?: string;
  percent?: number;
  deployment_id?: string;
  title?: string;
  detail?: string;
  request?: DecisionRequest;
  summary?: CompletionSummary;
  level?: AlertLevel;
  suggestion?: string;
}

export type CognitivePhase = 'perceive' | 'think' | 'decide' | 'act' | 'learn';

export const PHASE_ORDER: CognitivePhase[] = ['perceive', 'think', 'decide', 'act', 'learn'];

export const PHASE_LABELS: Record<CognitivePhase, string> = {
  perceive: 'Perceive',
  think: 'Think',
  decide: 'Decide',
  act: 'Act',
  learn: 'Learn',
};

export interface PhaseStatus {
  phase: CognitivePhase;
  status: 'pending' | 'running' | 'completed' | 'failed';
  tokens_used?: number;
  duration_ms?: number;
  result?: unknown;
}

export interface RunStep {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'skipped';
  progress?: number;
  message?: string;
  phase?: CognitivePhase;
}

export interface Run {
  id: string;
  intent: string;
  status: 'running' | 'completed' | 'failed' | 'cancelled';
  steps: RunStep[];
  phases: PhaseStatus[];
  started_at: string;
  completed_at?: string;
  total_tokens?: number;
  response?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'hydra';
  content: string;
  timestamp: string;
  run_id?: string;
  tokens_used?: number;
}

export type GlobeState = 'idle' | 'listening' | 'processing' | 'speaking' | 'error' | 'approval';

export interface HydraStatus {
  connected: boolean;
  icon_state: IconState;
  current_run?: Run;
  pending_approval?: DecisionRequest;
}

export const ICON_ANIMATIONS: Record<IconState, string> = {
  idle: 'animate-breathe',
  listening: 'animate-pulse-slow',
  working: 'animate-spin-slow',
  needs_attention: 'animate-pulse-slow',
  approval_needed: 'animate-bounce-gentle',
  success: '',
  error: '',
  offline: '',
};

export const ICON_COLORS: Record<IconState, string> = {
  idle: 'bg-hydra-idle',
  listening: 'bg-hydra-listening',
  working: 'bg-hydra-working',
  needs_attention: 'bg-hydra-attention',
  approval_needed: 'bg-hydra-approval',
  success: 'bg-hydra-success',
  error: 'bg-hydra-error',
  offline: 'bg-hydra-offline',
};

export const RISK_COLORS: Record<RiskLevel, string> = {
  none: 'bg-risk-none',
  low: 'bg-risk-low',
  medium: 'bg-risk-medium',
  high: 'bg-risk-high',
  critical: 'bg-risk-critical',
};
