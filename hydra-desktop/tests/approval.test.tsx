import { render, screen, fireEvent, act } from '@testing-library/react';
import { ApprovalCard } from '@/components/Approval/ApprovalCard';
import { DecisionRequest, DecisionResponse } from '@/types/hydra';

const baseRequest: DecisionRequest = {
  id: 'req-1',
  question: 'Deploy to production?',
  options: [
    { label: 'Yes', description: 'Deploy now', risk_level: 'medium' },
    { label: 'No', description: 'Cancel deployment', risk_level: 'none' },
  ],
};

describe('ApprovalCard', () => {
  test('test_approval_card_render', () => {
    const onSubmit = jest.fn();
    render(<ApprovalCard request={baseRequest} onSubmit={onSubmit} />);

    expect(screen.getByText('Deploy to production?')).toBeInTheDocument();
    expect(screen.getByText('Yes')).toBeInTheDocument();
    expect(screen.getByText('No')).toBeInTheDocument();
    expect(screen.getByText('Deploy now')).toBeInTheDocument();
    expect(screen.getByTestId('approval-card')).toBeInTheDocument();
  });

  test('clicking option submits decision', () => {
    const onSubmit = jest.fn();
    render(<ApprovalCard request={baseRequest} onSubmit={onSubmit} />);

    fireEvent.click(screen.getByText('Yes'));
    expect(onSubmit).toHaveBeenCalledWith({
      request_id: 'req-1',
      chosen_option: 0,
    } satisfies DecisionResponse);
  });

  test('max 4 options displayed', () => {
    const request: DecisionRequest = {
      id: 'req-2',
      question: 'Pick one',
      options: [
        { label: 'A' }, { label: 'B' }, { label: 'C' },
        { label: 'D' }, { label: 'E' },
      ],
    };
    const onSubmit = jest.fn();
    render(<ApprovalCard request={request} onSubmit={onSubmit} />);

    expect(screen.getByText('A')).toBeInTheDocument();
    expect(screen.getByText('D')).toBeInTheDocument();
    expect(screen.queryByText('E')).not.toBeInTheDocument();
  });

  test('test_approval_card_timeout', () => {
    jest.useFakeTimers();
    const onSubmit = jest.fn();
    const request: DecisionRequest = {
      ...baseRequest,
      timeout_seconds: 10,
      default: 1,
    };

    render(<ApprovalCard request={request} onSubmit={onSubmit} />);
    expect(screen.getByText('10s')).toBeInTheDocument();

    act(() => { jest.advanceTimersByTime(5000); });
    expect(screen.getByText('5s')).toBeInTheDocument();

    act(() => { jest.advanceTimersByTime(5000); });
    expect(onSubmit).toHaveBeenCalledWith({
      request_id: 'req-1',
      chosen_option: 1,
    });

    jest.useRealTimers();
  });
});
