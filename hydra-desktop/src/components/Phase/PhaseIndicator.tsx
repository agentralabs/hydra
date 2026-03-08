'use client';

import { PhaseStatus, PHASE_ORDER, PHASE_LABELS } from '@/types/hydra';

interface PhaseIndicatorProps {
  phases: PhaseStatus[];
  visible: boolean;
}

export function PhaseIndicator({ phases, visible }: PhaseIndicatorProps) {
  if (!visible || phases.length === 0) return null;

  const phaseMap = new Map(phases.map(p => [p.phase, p]));

  return (
    <div className="px-6 py-3 border-b border-zinc-800/50 bg-zinc-900/30 backdrop-blur-sm">
      <div className="flex items-center gap-1 max-w-xl mx-auto">
        {PHASE_ORDER.map((phase, i) => {
          const info = phaseMap.get(phase);
          const status = info?.status ?? 'pending';

          return (
            <div key={phase} className="flex items-center flex-1">
              {/* Phase node */}
              <div className="flex flex-col items-center gap-1 min-w-[60px]">
                <div className="flex items-center gap-1.5">
                  <div className={`phase-dot ${status}`} />
                  <span className={`text-xs font-medium tracking-wide ${
                    status === 'running'
                      ? 'text-indigo-300'
                      : status === 'completed'
                      ? 'text-emerald-400'
                      : status === 'failed'
                      ? 'text-red-400'
                      : 'text-zinc-500'
                  }`}>
                    {PHASE_LABELS[phase]}
                  </span>
                </div>
                {/* Metrics */}
                {(status === 'completed' || status === 'running') && info && (
                  <div className="flex gap-2 text-[10px] text-zinc-500">
                    {info.duration_ms !== undefined && (
                      <span>{info.duration_ms}ms</span>
                    )}
                    {info.tokens_used !== undefined && info.tokens_used > 0 && (
                      <span>{info.tokens_used}t</span>
                    )}
                  </div>
                )}
              </div>

              {/* Connector */}
              {i < PHASE_ORDER.length - 1 && (
                <div className={`phase-connector ${
                  status === 'completed'
                    ? 'completed'
                    : status === 'running'
                    ? 'active'
                    : 'pending'
                }`} />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
