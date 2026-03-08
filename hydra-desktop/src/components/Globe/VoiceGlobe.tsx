'use client';

import { GlobeState } from '@/types/hydra';

interface VoiceGlobeProps {
  state: GlobeState;
  onClick?: () => void;
  size?: 'sm' | 'md' | 'lg';
}

const SIZES = {
  sm: 'w-10 h-10',
  md: 'w-16 h-16',
  lg: 'w-20 h-20',
};

const LABELS: Record<GlobeState, string> = {
  idle: 'Click to activate voice',
  listening: 'Listening...',
  processing: 'Processing...',
  speaking: 'Speaking...',
  error: 'Error occurred',
  approval: 'Approval needed',
};

export function VoiceGlobe({ state, onClick, size = 'md' }: VoiceGlobeProps) {
  return (
    <button
      onClick={onClick}
      className={`voice-globe ${state} ${SIZES[size]}`}
      title={LABELS[state]}
      aria-label={`Voice: ${LABELS[state]}`}
    >
      {/* Inner icon */}
      <div className="absolute inset-0 flex items-center justify-center">
        <GlobeIcon state={state} />
      </div>
    </button>
  );
}

function GlobeIcon({ state }: { state: GlobeState }) {
  if (state === 'listening') {
    return (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round" className="opacity-80">
        <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
        <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
        <line x1="12" y1="19" x2="12" y2="23" />
      </svg>
    );
  }

  if (state === 'error') {
    return (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round" className="opacity-80">
        <circle cx="12" cy="12" r="10" />
        <line x1="15" y1="9" x2="9" y2="15" />
        <line x1="9" y1="9" x2="15" y2="15" />
      </svg>
    );
  }

  if (state === 'approval') {
    return (
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round" className="opacity-80">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
        <line x1="12" y1="9" x2="12" y2="13" />
        <line x1="12" y1="17" x2="12.01" y2="17" />
      </svg>
    );
  }

  // Default: waveform bars for idle/processing/speaking
  return (
    <div className="flex items-center gap-0.5 opacity-70">
      {[1, 2, 3, 4, 5].map(i => (
        <div
          key={i}
          className="w-0.5 bg-white/80 rounded-full"
          style={{
            height: state === 'idle' ? '8px' : `${6 + Math.sin(i * 1.2) * 6}px`,
            transition: 'height 0.3s ease',
          }}
        />
      ))}
    </div>
  );
}
