'use client';

import { useState } from 'react';
import { useHydra } from '@/hooks/useHydra';
import { MainWindow } from '@/components/Window/MainWindow';
import { CompanionWindow } from '@/components/Window/CompanionWindow';
import { LivingIcon } from '@/components/Icon/LivingIcon';

type ViewMode = 'tray' | 'companion' | 'main';

export default function Home() {
  const [viewMode, setViewMode] = useState<ViewMode>('main');
  const { status, messages, error, sendMessage, submitDecision, clearError } = useHydra();

  const handleIconClick = () => {
    setViewMode(prev => (prev === 'tray' ? 'companion' : 'tray'));
  };

  if (viewMode === 'main') {
    return (
      <MainWindow
        iconState={status.icon_state}
        messages={messages}
        currentRun={status.current_run}
        pendingApproval={status.pending_approval}
        error={error}
        onSendMessage={sendMessage}
        onSubmitDecision={submitDecision}
        onClearError={clearError}
      />
    );
  }

  return (
    <div className="fixed bottom-4 right-4 z-50">
      <LivingIcon state={status.icon_state} onClick={handleIconClick} />
      <CompanionWindow
        iconState={status.icon_state}
        currentRun={status.current_run}
        pendingApproval={status.pending_approval}
        onSubmitDecision={submitDecision}
        onExpand={() => setViewMode('main')}
        visible={viewMode === 'companion'}
      />
    </div>
  );
}
