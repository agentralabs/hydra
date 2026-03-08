'use client';

import { useState, useCallback, useEffect } from 'react';
import { useSSE } from './useSSE';
import { useIconState } from './useIconState';
import { HydraAPI } from '@/lib/api';
import {
  ChatMessage,
  CognitivePhase,
  DecisionRequest,
  DecisionResponse,
  PhaseStatus,
  Run,
  HydraStatus,
  SseEvent,
} from '@/types/hydra';

const api = new HydraAPI();

export function useHydra(baseUrl?: string) {
  const { connected, subscribe } = useSSE(baseUrl);
  const { state: iconState, transition } = useIconState(subscribe);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [currentRun, setCurrentRun] = useState<Run | null>(null);
  const [pendingApproval, setPendingApproval] = useState<DecisionRequest | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unsubs = [
      subscribe('run_started', (e: SseEvent) => {
        const data = e.data as { run_id: string; intent: string };
        setCurrentRun({
          id: data.run_id,
          intent: data.intent,
          status: 'running',
          steps: [],
          phases: [],
          started_at: e.timestamp,
        });
        setError(null);
      }),

      subscribe('step_started', (e: SseEvent) => {
        const data = e.data as { run_id: string; step_id: string; phase: CognitivePhase; sequence: number; description: string };
        setCurrentRun(prev => {
          if (!prev || prev.id !== data.run_id) return prev;
          // Update phase status
          const phases = [...prev.phases];
          const existing = phases.findIndex(p => p.phase === data.phase);
          const phaseStatus: PhaseStatus = { phase: data.phase, status: 'running' };
          if (existing >= 0) phases[existing] = phaseStatus;
          else phases.push(phaseStatus);
          return { ...prev, phases };
        });
      }),

      subscribe('step_completed', (e: SseEvent) => {
        const data = e.data as {
          run_id: string; step_id: string; phase: CognitivePhase;
          tokens_used?: number; duration_ms?: number; result?: unknown;
        };
        setCurrentRun(prev => {
          if (!prev || prev.id !== data.run_id) return prev;
          const phases = [...prev.phases];
          const existing = phases.findIndex(p => p.phase === data.phase);
          const phaseStatus: PhaseStatus = {
            phase: data.phase,
            status: 'completed',
            tokens_used: data.tokens_used,
            duration_ms: data.duration_ms,
            result: data.result,
          };
          if (existing >= 0) phases[existing] = phaseStatus;
          else phases.push(phaseStatus);
          return { ...prev, phases };
        });
      }),

      subscribe('step_progress', (e: SseEvent) => {
        const data = e.data as { run_id: string; step_id: string; phase: string; progress: number };
        setCurrentRun(prev => {
          if (!prev) return prev;
          const existing = prev.steps.findIndex(s => s.id === data.step_id);
          const step = { id: data.step_id, name: data.phase, status: 'running' as const, progress: data.progress };
          const steps = [...prev.steps];
          if (existing >= 0) steps[existing] = step;
          else steps.push(step);
          return { ...prev, steps };
        });
      }),

      subscribe('approval_required', (e: SseEvent) => {
        setPendingApproval(e.data as DecisionRequest);
      }),

      subscribe('run_completed', (e: SseEvent) => {
        const data = e.data as { run_id: string; tokens_used?: number; response?: string };

        setCurrentRun(prev => prev ? {
          ...prev,
          status: 'completed',
          completed_at: e.timestamp,
          total_tokens: data.tokens_used,
          response: data.response,
        } : prev);

        // Add the response as a hydra message
        if (data.response) {
          setMessages(prev => [...prev, {
            id: crypto.randomUUID(),
            role: 'hydra',
            content: data.response!,
            timestamp: e.timestamp,
            run_id: data.run_id,
            tokens_used: data.tokens_used,
          }]);
        }

        setPendingApproval(null);
      }),

      subscribe('run_error', (e: SseEvent) => {
        const data = e.data as { run_id: string; error: string };
        setCurrentRun(prev => prev ? { ...prev, status: 'failed' } : prev);
        setError(data.error);

        // Add error as hydra message
        setMessages(prev => [...prev, {
          id: crypto.randomUUID(),
          role: 'hydra',
          content: `Error: ${data.error}`,
          timestamp: e.timestamp,
          run_id: data.run_id,
        }]);

        setPendingApproval(null);
      }),
    ];

    return () => unsubs.forEach(fn => fn());
  }, [subscribe]);

  const sendMessage = useCallback(async (content: string) => {
    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    };
    setMessages(prev => [...prev, userMsg]);

    try {
      await api.sendMessage(content);
      // Response will come via SSE run_completed event
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
    }
  }, []);

  const submitDecision = useCallback(async (response: DecisionResponse) => {
    try {
      await api.submitDecision(response);
      setPendingApproval(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit decision');
    }
  }, []);

  const status: HydraStatus = {
    connected,
    icon_state: iconState,
    current_run: currentRun ?? undefined,
    pending_approval: pendingApproval ?? undefined,
  };

  return {
    status,
    messages,
    error,
    sendMessage,
    submitDecision,
    transition,
    clearError: () => setError(null),
  };
}
