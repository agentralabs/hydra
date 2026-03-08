'use client';

import { IconState } from '@/types/hydra';
import { IconStateIndicator, ICON_LABELS } from './IconStates';

interface LivingIconProps {
  state: IconState;
  onClick?: () => void;
  onContextMenu?: (e: React.MouseEvent) => void;
}

export function LivingIcon({ state, onClick, onContextMenu }: LivingIconProps) {
  return (
    <button
      onClick={onClick}
      onContextMenu={onContextMenu}
      className="group relative flex items-center gap-2 p-2 rounded-xl
                 hover:bg-white/10 transition-colors cursor-pointer select-none"
      title={ICON_LABELS[state]}
      aria-label={`Hydra status: ${ICON_LABELS[state]}`}
    >
      <IconStateIndicator state={state} size="md" />
      <span className="text-sm text-gray-300 opacity-0 group-hover:opacity-100 transition-opacity">
        {ICON_LABELS[state]}
      </span>
    </button>
  );
}
