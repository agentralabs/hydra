import { render, screen } from '@testing-library/react';
import { CompanionWindow } from '@/components/Window/CompanionWindow';

describe('CompanionWindow', () => {
  const defaultProps = {
    iconState: 'idle' as const,
    onSubmitDecision: jest.fn(),
    onExpand: jest.fn(),
    visible: true,
  };

  test('test_companion_window_toggle', () => {
    const { rerender } = render(<CompanionWindow {...defaultProps} visible={false} />);
    expect(screen.queryByTestId('companion-window')).not.toBeInTheDocument();

    rerender(<CompanionWindow {...defaultProps} visible={true} />);
    expect(screen.getByTestId('companion-window')).toBeInTheDocument();
  });

  test('shows ready message when idle', () => {
    render(<CompanionWindow {...defaultProps} />);
    expect(screen.getByText('Ready to help')).toBeInTheDocument();
  });

  test('shows connecting when offline', () => {
    render(<CompanionWindow {...defaultProps} iconState="offline" />);
    expect(screen.getByText('Connecting...')).toBeInTheDocument();
  });

  test('shows current run info', () => {
    render(
      <CompanionWindow
        {...defaultProps}
        currentRun={{
          id: 'run-1',
          intent: 'Refactor auth module',
          status: 'running',
          steps: [{ id: 's1', name: 'Analyzing code', status: 'running', progress: 50 }],
          phases: [],
          started_at: new Date().toISOString(),
        }}
      />,
    );
    expect(screen.getByText('Refactor auth module')).toBeInTheDocument();
    expect(screen.getByText('Analyzing code')).toBeInTheDocument();
  });
});
