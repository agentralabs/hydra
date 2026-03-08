'use client';

import { useState } from 'react';
import { ChatMessage, DecisionRequest, DecisionResponse, IconState, Run, GlobeState, PhaseStatus } from '@/types/hydra';
import { ChatInterface } from '@/components/Chat/ChatInterface';
import { ApprovalCard } from '@/components/Approval/ApprovalCard';
import { PhaseIndicator } from '@/components/Phase/PhaseIndicator';
import { VoiceGlobe } from '@/components/Globe/VoiceGlobe';

type Tab = 'chat' | 'settings';

interface MainWindowProps {
  iconState: IconState;
  messages: ChatMessage[];
  currentRun?: Run;
  pendingApproval?: DecisionRequest;
  error?: string | null;
  onSendMessage: (content: string) => void;
  onSubmitDecision: (response: DecisionResponse) => void;
  onClearError: () => void;
}

function iconToGlobe(icon: IconState, hasRun: boolean): GlobeState {
  if (icon === 'error') return 'error';
  if (icon === 'approval_needed') return 'approval';
  if (icon === 'working' || hasRun) return 'processing';
  if (icon === 'listening') return 'listening';
  return 'idle';
}

export function MainWindow({
  iconState,
  messages,
  currentRun,
  pendingApproval,
  error,
  onSendMessage,
  onSubmitDecision,
  onClearError,
}: MainWindowProps) {
  const [activeTab, setActiveTab] = useState<Tab>('chat');
  const globeState = iconToGlobe(iconState, currentRun?.status === 'running');
  const isProcessing = currentRun?.status === 'running';

  return (
    <div className="flex flex-col h-screen" data-testid="main-window">
      {/* Header */}
      <header className="flex items-center justify-between px-5 py-3 border-b border-zinc-800/50 bg-zinc-900/50 backdrop-blur-sm">
        <div className="flex items-center gap-3">
          <VoiceGlobe state={globeState} size="sm" />
          <div>
            <h1 className="text-sm font-semibold tracking-wide text-white">HYDRA</h1>
            <p className="text-[10px] text-zinc-500 uppercase tracking-widest">
              {isProcessing ? 'Processing' : iconState === 'offline' ? 'Offline' : 'Ready'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {/* Connection indicator */}
          <div className="flex items-center gap-1.5">
            <div className={`w-1.5 h-1.5 rounded-full ${
              iconState === 'offline' ? 'bg-red-400' : 'bg-emerald-400'
            }`} />
            <span className="text-[10px] text-zinc-500">
              {iconState === 'offline' ? 'Disconnected' : 'Connected'}
            </span>
          </div>

          {/* Tab nav */}
          <nav className="flex bg-zinc-800/40 rounded-lg p-0.5">
            {(['chat', 'settings'] as Tab[]).map(tab => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className={`px-3 py-1 text-xs rounded-md capitalize transition-all duration-200 ${
                  activeTab === tab
                    ? 'bg-zinc-700 text-white shadow-sm'
                    : 'text-zinc-500 hover:text-zinc-300'
                }`}
              >
                {tab}
              </button>
            ))}
          </nav>
        </div>
      </header>

      {/* Error banner */}
      {error && (
        <div className="flex items-center justify-between px-5 py-2 bg-red-950/40 border-b border-red-900/30">
          <p className="text-xs text-red-300">{error}</p>
          <button onClick={onClearError} className="text-red-400 hover:text-red-200 text-xs transition-colors">
            Dismiss
          </button>
        </div>
      )}

      {/* Phase indicator */}
      <PhaseIndicator
        phases={currentRun?.phases ?? []}
        visible={isProcessing || (currentRun?.status === 'completed' && (currentRun?.phases?.length ?? 0) > 0)}
      />

      {/* Pending approval */}
      {pendingApproval && (
        <div className="px-5 py-3 border-b border-zinc-800/50 bg-amber-950/10">
          <div className="max-w-2xl mx-auto">
            <ApprovalCard request={pendingApproval} onSubmit={onSubmitDecision} />
          </div>
        </div>
      )}

      {/* Content */}
      <main className="flex-1 overflow-hidden">
        {activeTab === 'chat' && (
          <ChatInterface
            messages={messages}
            onSend={onSendMessage}
            disabled={iconState === 'offline'}
            isProcessing={isProcessing}
          />
        )}
        {activeTab === 'settings' && (
          <div className="p-6 max-w-2xl mx-auto">
            <h2 className="text-sm font-medium text-white mb-4">Settings</h2>
            <div className="space-y-3 text-sm">
              <SettingsRow label="Connection" value={iconState === 'offline' ? 'Disconnected' : 'Connected'}
                valueColor={iconState === 'offline' ? 'text-red-400' : 'text-emerald-400'} />
              <SettingsRow label="Server" value="localhost:7777" />
              <SettingsRow label="Tokens Used" value={currentRun?.total_tokens?.toString() ?? '0'} />
            </div>
          </div>
        )}
      </main>

      {/* Status bar */}
      {currentRun && (
        <footer className="px-5 py-2 border-t border-zinc-800/50 bg-zinc-900/30 backdrop-blur-sm
                          flex items-center gap-3 text-xs text-zinc-500">
          {currentRun.status === 'running' && (
            <span className="w-1.5 h-1.5 rounded-full bg-indigo-400 animate-pulse" />
          )}
          {currentRun.status === 'completed' && (
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
          )}
          {currentRun.status === 'failed' && (
            <span className="w-1.5 h-1.5 rounded-full bg-red-400" />
          )}
          <span className="truncate flex-1">{currentRun.intent}</span>
          {currentRun.total_tokens !== undefined && currentRun.total_tokens > 0 && (
            <span className="text-zinc-600">{currentRun.total_tokens} tokens</span>
          )}
          <span className="capitalize text-zinc-600">{currentRun.status}</span>
        </footer>
      )}
    </div>
  );
}

function SettingsRow({ label, value, valueColor }: { label: string; value: string; valueColor?: string }) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-zinc-800/30">
      <span className="text-zinc-400">{label}</span>
      <span className={valueColor ?? 'text-zinc-500'}>{value}</span>
    </div>
  );
}
