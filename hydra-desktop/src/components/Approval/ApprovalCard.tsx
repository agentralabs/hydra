'use client';

import { useState, useEffect, useCallback } from 'react';
import { DecisionRequest, DecisionResponse, RISK_COLORS, RiskLevel } from '@/types/hydra';

interface ApprovalCardProps {
  request: DecisionRequest;
  onSubmit: (response: DecisionResponse) => void;
}

export function ApprovalCard({ request, onSubmit }: ApprovalCardProps) {
  const [remaining, setRemaining] = useState(request.timeout_seconds ?? 0);
  const hasTimeout = (request.timeout_seconds ?? 0) > 0;

  useEffect(() => {
    if (!hasTimeout) return;
    setRemaining(request.timeout_seconds!);

    const interval = setInterval(() => {
      setRemaining(prev => {
        if (prev <= 1) {
          clearInterval(interval);
          if (request.default !== undefined) {
            onSubmit({ request_id: request.id, chosen_option: request.default! });
          }
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(interval);
  }, [request.id, request.timeout_seconds, request.default, hasTimeout, onSubmit]);

  const handleChoose = useCallback(
    (index: number) => {
      onSubmit({ request_id: request.id, chosen_option: index });
    },
    [request.id, onSubmit],
  );

  const maxOptions = request.options.slice(0, 4);

  return (
    <div
      role="dialog"
      aria-label="Approval required"
      className="bg-gray-800 border border-gray-600 rounded-xl p-4 shadow-lg max-w-md"
      data-testid="approval-card"
    >
      <p className="text-white text-sm font-medium mb-3">{request.question}</p>

      <div className="space-y-2">
        {maxOptions.map((option, i) => {
          const riskLevel = option.risk_level ?? 'none';
          return (
            <button
              key={i}
              onClick={() => handleChoose(i)}
              className={`
                w-full text-left px-3 py-2 rounded-lg text-sm
                bg-gray-700 hover:bg-gray-600 transition-colors
                flex items-center justify-between
              `}
              aria-label={`${option.label}${option.description ? `: ${option.description}` : ''}`}
            >
              <div>
                <span className="text-white font-medium">{option.label}</span>
                {option.description && (
                  <span className="block text-gray-400 text-xs mt-0.5">{option.description}</span>
                )}
              </div>
              <div className="flex items-center gap-2">
                {option.keyboard_shortcut && (
                  <kbd className="text-xs text-gray-400 bg-gray-600 px-1.5 py-0.5 rounded">
                    {option.keyboard_shortcut}
                  </kbd>
                )}
                <RiskDot level={riskLevel} />
              </div>
            </button>
          );
        })}
      </div>

      {hasTimeout && (
        <div className="mt-3 flex items-center gap-2">
          <div className="flex-1 bg-gray-700 rounded-full h-1.5 overflow-hidden">
            <div
              className="bg-hydra-approval h-full transition-all duration-1000 ease-linear"
              style={{ width: `${(remaining / (request.timeout_seconds ?? 1)) * 100}%` }}
              role="progressbar"
              aria-valuenow={remaining}
              aria-valuemax={request.timeout_seconds}
              aria-label="Time remaining"
            />
          </div>
          <span className="text-xs text-gray-400 tabular-nums">{remaining}s</span>
        </div>
      )}
    </div>
  );
}

function RiskDot({ level }: { level: RiskLevel }) {
  return (
    <span
      className={`w-2 h-2 rounded-full ${RISK_COLORS[level]}`}
      title={`Risk: ${level}`}
      aria-label={`Risk level: ${level}`}
    />
  );
}
