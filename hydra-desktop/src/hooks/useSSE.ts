'use client';

import { useEffect, useRef, useCallback, useState } from 'react';
import { HydraSSEClient, SseCallback } from '@/lib/sse';
import { SseEvent, SseEventType } from '@/types/hydra';

const DEFAULT_URL = 'http://localhost:7777';

export function useSSE(baseUrl: string = DEFAULT_URL) {
  const clientRef = useRef<HydraSSEClient | null>(null);
  const [connected, setConnected] = useState(false);
  const [lastEvent, setLastEvent] = useState<SseEvent | null>(null);

  useEffect(() => {
    const client = new HydraSSEClient(baseUrl);
    clientRef.current = client;

    const unsubReady = client.on('system_ready', () => setConnected(true));
    const unsubShutdown = client.on('system_shutdown', () => setConnected(false));
    const unsubAll = client.on('*', (event) => setLastEvent(event));

    client.connect();

    return () => {
      unsubReady();
      unsubShutdown();
      unsubAll();
      client.disconnect();
    };
  }, [baseUrl]);

  const subscribe = useCallback(
    (eventType: SseEventType | '*', callback: SseCallback): (() => void) => {
      return clientRef.current?.on(eventType, callback) ?? (() => {});
    },
    [],
  );

  return { connected, lastEvent, subscribe };
}
