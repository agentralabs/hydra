'use client';

import { IconState, Run, DecisionRequest, DecisionResponse } from '@/types/hydra';
import { LivingIcon } from '@/components/Icon/LivingIcon';
import { ApprovalCard } from '@/components/Approval/ApprovalCard';

interface CompanionWindowProps {
  iconState: IconState;
  currentRun?: Run;
  pendingApproval?: DecisionRequest;
  onSubmitDecision: (response: DecisionResponse) => void;
  onExpand: () => void;
  visible: boolean;
}

export function CompanionWindow({
  iconState,
  currentRun,
  pendingApproval,
  onSubmitDecision,
  onExpand,
  visible,
}: CompanionWindowProps) {
  if (!visible) return null;

  return (
    <div
      data-testid="companion-window"
      className="fixed bottom-20 right-4 w-80 bg-gray-900 border border-gray-700
                 rounded-2xl shadow-2xl overflow-hidden z-50"
    >
      <div className="flex items-center justify-between px-4 py-3 border-b border-gray-800">
        <LivingIcon state={iconState} />
        <button
          onClick={onExpand}
          className="text-xs text-gray-400 hover:text-white transition-colors"
          aria-label="Expand to full window"
        >
          Expand
        </button>
      </div>

      <div className="p-4 space-y-3 max-h-96 overflow-y-auto">
        {currentRun && (
          <div className="text-sm">
            <p className="text-gray-400 text-xs uppercase tracking-wide mb-1">Current Task</p>
            <p className="text-white">{currentRun.intent}</p>
            {currentRun.steps.length > 0 && (
              <div className="mt-2 space-y-1">
                {currentRun.steps.map(step => (
                  <div key={step.id} className="flex items-center gap-2 text-xs">
                    <span className={`w-1.5 h-1.5 rounded-full ${
                      step.status === 'running' ? 'bg-blue-400 animate-pulse' :
                      step.status === 'completed' ? 'bg-green-400' :
                      step.status === 'failed' ? 'bg-red-400' : 'bg-gray-500'
                    }`} />
                    <span className="text-gray-300 truncate">{step.name}</span>
                    {step.progress !== undefined && (
                      <span className="text-gray-500 ml-auto">{Math.round(step.progress)}%</span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {pendingApproval && (
          <ApprovalCard request={pendingApproval} onSubmit={onSubmitDecision} />
        )}

        {!currentRun && !pendingApproval && (
          <p className="text-gray-500 text-sm text-center py-4">
            {iconState === 'offline' ? 'Connecting...' : 'Ready to help'}
          </p>
        )}
      </div>
    </div>
  );
}
