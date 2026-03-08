import { SseEvent, SseEventType } from '@/types/hydra';

export type SseCallback = (event: SseEvent) => void;

const HEARTBEAT_INTERVAL_MS = 30_000;
const HEARTBEAT_TIMEOUT_MS = 60_000;
const MAX_RECONNECT_DELAY_MS = 30_000;

export class HydraSSEClient {
  private eventSource: EventSource | null = null;
  private listeners: Map<string, Set<SseCallback>> = new Map();
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private lastHeartbeat = 0;
  private heartbeatChecker: ReturnType<typeof setInterval> | null = null;
  private _connected = false;
  private url: string;

  constructor(baseUrl: string) {
    this.url = `${baseUrl}/events`;
  }

  get connected(): boolean {
    return this._connected;
  }

  connect(): void {
    this.cleanup();

    try {
      this.eventSource = new EventSource(this.url);
    } catch {
      this.handleDisconnect();
      return;
    }

    this.eventSource.onopen = () => {
      this._connected = true;
      this.reconnectAttempts = 0;
      this.lastHeartbeat = Date.now();
      this.startHeartbeatMonitor();
      this.emit({ event_type: 'system_ready', data: {}, timestamp: new Date().toISOString() });
    };

    this.eventSource.onerror = () => {
      this.handleDisconnect();
    };

    const eventTypes: SseEventType[] = [
      'run_started', 'step_started', 'step_progress', 'step_completed',
      'approval_required', 'run_completed', 'run_error',
      'heartbeat', 'system_ready', 'system_shutdown',
    ];

    for (const type of eventTypes) {
      this.eventSource.addEventListener(type, (e: MessageEvent) => {
        const event: SseEvent = {
          event_type: type,
          data: JSON.parse(e.data),
          timestamp: new Date().toISOString(),
        };
        if (type === 'heartbeat') {
          this.lastHeartbeat = Date.now();
        }
        this.emit(event);
      });
    }
  }

  disconnect(): void {
    this.cleanup();
    this._connected = false;
  }

  on(eventType: SseEventType | '*', callback: SseCallback): () => void {
    const key = eventType;
    if (!this.listeners.has(key)) {
      this.listeners.set(key, new Set());
    }
    this.listeners.get(key)!.add(callback);
    return () => {
      this.listeners.get(key)?.delete(callback);
    };
  }

  private emit(event: SseEvent): void {
    this.listeners.get(event.event_type)?.forEach(cb => cb(event));
    this.listeners.get('*')?.forEach(cb => cb(event));
  }

  private handleDisconnect(): void {
    this.cleanup();
    this._connected = false;
    this.emit({
      event_type: 'system_shutdown',
      data: { reason: 'disconnected' },
      timestamp: new Date().toISOString(),
    });
    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    const delay = Math.min(
      1000 * Math.pow(2, this.reconnectAttempts),
      MAX_RECONNECT_DELAY_MS,
    );
    this.reconnectAttempts++;
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }

  private startHeartbeatMonitor(): void {
    this.stopHeartbeatMonitor();
    this.heartbeatChecker = setInterval(() => {
      const elapsed = Date.now() - this.lastHeartbeat;
      if (elapsed > HEARTBEAT_TIMEOUT_MS) {
        this.handleDisconnect();
      }
    }, HEARTBEAT_INTERVAL_MS);
  }

  private stopHeartbeatMonitor(): void {
    if (this.heartbeatChecker) {
      clearInterval(this.heartbeatChecker);
      this.heartbeatChecker = null;
    }
  }

  private cleanup(): void {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.stopHeartbeatMonitor();
  }
}
