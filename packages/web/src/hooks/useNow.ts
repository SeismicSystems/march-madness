import { useSyncExternalStore } from "react";

type TimeStore = {
  getSnapshot: () => number;
  subscribe: (listener: () => void) => () => void;
};

const stores = new Map<number, TimeStore>();

const createTimeStore = (intervalMs: number): TimeStore => {
  let intervalId: ReturnType<typeof setInterval> | null = null;
  const listeners = new Set<() => void>();

  const emit = () => {
    for (const listener of listeners) {
      listener();
    }
  };

  return {
    getSnapshot: () => Date.now(),
    subscribe: (listener) => {
      listeners.add(listener);
      if (intervalId === null) {
        intervalId = setInterval(emit, intervalMs);
      }

      return () => {
        listeners.delete(listener);
        if (listeners.size === 0 && intervalId !== null) {
          clearInterval(intervalId);
          intervalId = null;
        }
      };
    },
  };
};

const getTimeStore = (intervalMs: number): TimeStore => {
  const existingStore = stores.get(intervalMs);
  if (existingStore) return existingStore;

  const store = createTimeStore(intervalMs);
  stores.set(intervalMs, store);
  return store;
};

export function useNow(intervalMs = 1000): number {
  const store = getTimeStore(intervalMs);
  return useSyncExternalStore(
    store.subscribe,
    store.getSnapshot,
    store.getSnapshot,
  );
}
