import { useEffect, useRef } from "react";

type DebugValues = Record<string, unknown>;

const DEBUG_FLAG = "mm-debug-wallet";

export const isDebugEnabled = (): boolean => {
  if (!import.meta.env.DEV || typeof window === "undefined") {
    return false;
  }

  const params = new URLSearchParams(window.location.search);
  if (params.get(DEBUG_FLAG) === "1") {
    return true;
  }

  try {
    return window.localStorage.getItem(DEBUG_FLAG) === "1";
  } catch {
    return false;
  }
};

export const debugLog = (label: string, payload?: unknown) => {
  if (!isDebugEnabled()) return;
  console.log(`[mm-debug] ${label}`, payload);
};

const formatValue = (value: unknown): unknown => {
  if (typeof value === "bigint") return value.toString();
  if (value instanceof Error) return value.message;
  return value;
};

export function useDebugValueChanges(label: string, values: DebugValues) {
  const previousRef = useRef<DebugValues | null>(null);

  useEffect(() => {
    if (!isDebugEnabled()) return;

    const nextEntries = Object.entries(values).map(([key, value]) => [
      key,
      formatValue(value),
    ]);
    const nextValues = Object.fromEntries(nextEntries);
    const previous = previousRef.current;

    if (!previous) {
      console.log(`[mm-debug] ${label} init`, nextValues);
      previousRef.current = nextValues;
      return;
    }

    const changedEntries = Object.entries(nextValues).filter(
      ([key, value]) => !Object.is(previous[key], value),
    );

    if (changedEntries.length === 0) return;

    console.groupCollapsed(
      `[mm-debug] ${label} changed (${changedEntries
        .map(([key]) => key)
        .join(", ")})`,
    );
    for (const [key, value] of changedEntries) {
      console.log(key, {
        from: previous[key],
        to: value,
      });
    }
    console.groupEnd();

    previousRef.current = nextValues;
  }, [label, values]);
}
