import { HydraSSEClient } from '@/lib/sse';
import { SseEvent } from '@/types/hydra';

// Mock EventSource
class MockEventSource {
  static instances: MockEventSource[] = [];
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  listeners: Map<string, ((e: MessageEvent) => void)[]> = new Map();
  closed = false;

  constructor(public url: string) {
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, cb: (e: MessageEvent) => void) {
    if (!this.listeners.has(type)) this.listeners.set(type, []);
    this.listeners.get(type)!.push(cb);
  }

  close() {
    this.closed = true;
  }

  simulateOpen() {
    this.onopen?.();
  }

  simulateError() {
    this.onerror?.();
  }

  simulateEvent(type: string, data: unknown) {
    const event = { data: JSON.stringify(data) } as MessageEvent;
    this.listeners.get(type)?.forEach(cb => cb(event));
  }
}

(globalThis as unknown as { EventSource: typeof MockEventSource }).EventSource = MockEventSource as unknown as typeof EventSource;

beforeEach(() => {
  MockEventSource.instances = [];
  jest.useFakeTimers();
});

afterEach(() => {
  jest.useRealTimers();
});

describe('HydraSSEClient', () => {
  test('test_sse_connection', () => {
    const client = new HydraSSEClient('http://localhost:7777');
    expect(client.connected).toBe(false);

    client.connect();
    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe('http://localhost:7777/events');

    MockEventSource.instances[0].simulateOpen();
    expect(client.connected).toBe(true);

    client.disconnect();
    expect(client.connected).toBe(false);
  });

  test('test_sse_event_handling', () => {
    const client = new HydraSSEClient('http://localhost:7777');
    const received: SseEvent[] = [];

    client.connect();
    MockEventSource.instances[0].simulateOpen();

    client.on('run_started', (e) => received.push(e));
    MockEventSource.instances[0].simulateEvent('run_started', { run_id: 'r1', intent: 'test' });

    expect(received).toHaveLength(1);
    expect(received[0].event_type).toBe('run_started');
    expect((received[0].data as { run_id: string }).run_id).toBe('r1');

    client.disconnect();
  });

  test('test_sse_reconnect', () => {
    const client = new HydraSSEClient('http://localhost:7777');
    client.connect();
    MockEventSource.instances[0].simulateOpen();
    expect(client.connected).toBe(true);

    // Simulate disconnect
    MockEventSource.instances[0].simulateError();
    expect(client.connected).toBe(false);

    // Advance past reconnect delay (1s for first attempt)
    jest.advanceTimersByTime(1500);
    expect(MockEventSource.instances).toHaveLength(2);

    // Second reconnect with exponential backoff
    MockEventSource.instances[1].simulateError();
    jest.advanceTimersByTime(2500);
    expect(MockEventSource.instances).toHaveLength(3);

    client.disconnect();
  });

  test('test_sse_heartbeat_timeout', () => {
    const client = new HydraSSEClient('http://localhost:7777');
    const shutdowns: SseEvent[] = [];
    client.on('system_shutdown', (e) => shutdowns.push(e));

    client.connect();
    MockEventSource.instances[0].simulateOpen();
    expect(client.connected).toBe(true);

    // No heartbeat for 90s+ (3 check intervals) should trigger disconnect
    jest.advanceTimersByTime(91000);
    expect(client.connected).toBe(false);
    expect(shutdowns.length).toBeGreaterThan(0);

    client.disconnect();
  });

  test('wildcard listener receives all events', () => {
    const client = new HydraSSEClient('http://localhost:7777');
    const received: SseEvent[] = [];

    client.connect();
    MockEventSource.instances[0].simulateOpen();
    client.on('*', (e) => received.push(e));

    MockEventSource.instances[0].simulateEvent('run_started', { run_id: 'r1' });
    MockEventSource.instances[0].simulateEvent('heartbeat', { status: 'alive' });

    // system_ready from onopen + 2 events
    expect(received.length).toBeGreaterThanOrEqual(2);

    client.disconnect();
  });

  test('unsubscribe stops receiving events', () => {
    const client = new HydraSSEClient('http://localhost:7777');
    const received: SseEvent[] = [];

    client.connect();
    MockEventSource.instances[0].simulateOpen();

    const unsub = client.on('heartbeat', (e) => received.push(e));
    MockEventSource.instances[0].simulateEvent('heartbeat', {});
    expect(received).toHaveLength(1);

    unsub();
    MockEventSource.instances[0].simulateEvent('heartbeat', {});
    expect(received).toHaveLength(1);

    client.disconnect();
  });
});
