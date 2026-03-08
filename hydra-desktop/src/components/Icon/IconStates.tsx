'use client';

import { IconState, ICON_COLORS, ICON_ANIMATIONS } from '@/types/hydra';

interface IconStateIndicatorProps {
  state: IconState;
  size?: 'sm' | 'md' | 'lg';
}

const SIZES = {
  sm: 'w-4 h-4',
  md: 'w-8 h-8',
  lg: 'w-16 h-16',
};

const LABELS: Record<IconState, string> = {
  idle: 'Ready',
  listening: 'Listening',
  working: 'Working',
  needs_attention: 'Needs Attention',
  approval_needed: 'Waiting for Approval',
  success: 'Done',
  error: 'Error',
  offline: 'Offline',
};

export function IconStateIndicator({ state, size = 'md' }: IconStateIndicatorProps) {
  const isOffline = state === 'offline';

  return (
    <div
      role="status"
      aria-label={LABELS[state]}
      data-state={state}
      className={`
        rounded-full transition-colors duration-500
        ${SIZES[size]}
        ${ICON_COLORS[state]}
        ${ICON_ANIMATIONS[state]}
        ${isOffline ? 'ring-2 ring-gray-400 bg-transparent' : ''}
      `}
    />
  );
}

export { LABELS as ICON_LABELS };
