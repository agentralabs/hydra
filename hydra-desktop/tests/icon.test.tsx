import { render, screen } from '@testing-library/react';
import { IconStateIndicator } from '@/components/Icon/IconStates';
import { IconState, ICON_COLORS, ICON_ANIMATIONS } from '@/types/hydra';

const ALL_STATES: IconState[] = [
  'idle', 'listening', 'working', 'needs_attention',
  'approval_needed', 'success', 'error', 'offline',
];

describe('IconStateIndicator', () => {
  test('test_icon_all_8_states', () => {
    for (const state of ALL_STATES) {
      const { unmount } = render(<IconStateIndicator state={state} />);
      const el = screen.getByRole('status');
      expect(el).toBeInTheDocument();
      expect(el).toHaveAttribute('data-state', state);
      unmount();
    }
  });

  test('test_icon_state_transitions', () => {
    const { rerender } = render(<IconStateIndicator state="idle" />);
    expect(screen.getByRole('status')).toHaveAttribute('data-state', 'idle');

    rerender(<IconStateIndicator state="working" />);
    expect(screen.getByRole('status')).toHaveAttribute('data-state', 'working');

    rerender(<IconStateIndicator state="success" />);
    expect(screen.getByRole('status')).toHaveAttribute('data-state', 'success');

    rerender(<IconStateIndicator state="error" />);
    expect(screen.getByRole('status')).toHaveAttribute('data-state', 'error');

    rerender(<IconStateIndicator state="offline" />);
    expect(screen.getByRole('status')).toHaveAttribute('data-state', 'offline');
  });

  test('offline state renders hollow ring', () => {
    render(<IconStateIndicator state="offline" />);
    const el = screen.getByRole('status');
    expect(el.className).toContain('ring-2');
    expect(el.className).toContain('bg-transparent');
  });

  test('all states have correct color classes', () => {
    for (const state of ALL_STATES) {
      expect(ICON_COLORS[state]).toBeDefined();
      expect(typeof ICON_ANIMATIONS[state]).toBe('string');
    }
  });
});
