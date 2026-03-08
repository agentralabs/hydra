'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { IconState, SseEvent, SseEventType } from '@/types/hydra';

const TRANSIENT_DURATION_MS: Partial<Record<IconState, number>> = {
  success: 2000,
};

export function useIconState(subscribe: (type: SseEventType | '*', cb: (e: SseEvent) => void) => () => void) {
  const [state, setState] = useState<IconState>('offline');
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const transition = useCallback((newState: IconState) => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }

    // Offline can only go to idle
    setState(prev => {
      if (prev === 'offline' && newState !== 'idle') return prev;
      return newState;
    });

    const duration = TRANSIENT_DURATION_MS[newState];
    if (duration) {
      timerRef.current = setTimeout(() => {
        setState('idle');
        timerRef.current = null;
      }, duration);
    }
  }, []);

  useEffect(() => {
    const unsubs = [
      subscribe('system_ready', () => transition('idle')),
      subscribe('system_shutdown', () => transition('offline')),
      subscribe('run_started', () => transition('working')),
      subscribe('step_progress', () => transition('working')),
      subscribe('approval_required', () => transition('approval_needed')),
      subscribe('run_completed', () => transition('success')),
      subscribe('run_error', () => transition('error')),
    ];

    return () => unsubs.forEach(fn => fn());
  }, [subscribe, transition]);

  return { state, transition };
}
