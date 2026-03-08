import { render, screen, fireEvent } from '@testing-library/react';
import { MainWindow } from '@/components/Window/MainWindow';
import { DecisionRequest } from '@/types/hydra';

describe('Integration tests', () => {
  const baseProps = {
    iconState: 'idle' as const,
    messages: [],
    onSendMessage: jest.fn(),
    onSubmitDecision: jest.fn(),
    onClearError: jest.fn(),
  };

  test('test_full_run_flow', () => {
    const { rerender } = render(<MainWindow {...baseProps} />);
    expect(screen.getByTestId('main-window')).toBeInTheDocument();

    // Run starts
    rerender(
      <MainWindow
        {...baseProps}
        iconState="working"
        currentRun={{
          id: 'run-1',
          intent: 'Add login page',
          status: 'running',
          steps: [{ id: 's1', name: 'Planning', status: 'running', progress: 30 }],
          phases: [],
          started_at: new Date().toISOString(),
        }}
      />,
    );
    expect(screen.getByText('Add login page')).toBeInTheDocument();

    // Run completes
    rerender(
      <MainWindow
        {...baseProps}
        iconState="success"
        currentRun={{
          id: 'run-1',
          intent: 'Add login page',
          status: 'completed',
          steps: [{ id: 's1', name: 'Planning', status: 'completed', progress: 100 }],
          phases: [],
          started_at: new Date().toISOString(),
          completed_at: new Date().toISOString(),
        }}
      />,
    );
    expect(screen.getByText('completed')).toBeInTheDocument();
  });

  test('test_approval_flow', () => {
    const onSubmit = jest.fn();
    const approval: DecisionRequest = {
      id: 'dec-1',
      question: 'Push to main?',
      options: [
        { label: 'Push', risk_level: 'high' },
        { label: 'Cancel', risk_level: 'none' },
      ],
    };

    render(
      <MainWindow
        {...baseProps}
        iconState="approval_needed"
        pendingApproval={approval}
        onSubmitDecision={onSubmit}
      />,
    );

    expect(screen.getByText('Push to main?')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Cancel'));
    expect(onSubmit).toHaveBeenCalledWith({
      request_id: 'dec-1',
      chosen_option: 1,
    });
  });

  test('test_error_display', () => {
    const onClear = jest.fn();
    render(
      <MainWindow
        {...baseProps}
        iconState="error"
        error="Connection lost. Try again in a moment."
      />,
    );

    expect(screen.getByText('Connection lost. Try again in a moment.')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Dismiss'));
    // onClearError is from baseProps.onClearError (jest.fn)
  });

  test('tabs switch correctly', () => {
    render(<MainWindow {...baseProps} />);

    fireEvent.click(screen.getByText('settings'));
    expect(screen.getByText('Settings')).toBeInTheDocument();

    fireEvent.click(screen.getByText('chat'));
    expect(screen.getByPlaceholderText('Ask Hydra anything...')).toBeInTheDocument();
  });
});
